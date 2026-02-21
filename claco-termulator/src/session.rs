use alacritty_terminal::{
    Term, event::VoidListener, grid::Dimensions, term::Config, vte::ansi::Processor,
};
use claco_pty::{
    process::{Child, spawn_child},
    pty::{open_pty, set_pty_size},
};
use futures::SinkExt;
use std::{
    io,
    os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, OwnedFd},
    sync::Arc,
    time::Duration,
};
use tokio::io::AsyncReadExt;
use tokio::net::UnixListener;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio_serde::formats::Bincode;
use tokio_util::codec::LengthDelimitedCodec;

use crate::{frame_conv::term_to_frame, ipc::Frame};
use regex::Regex;

#[derive(Clone, Debug)]
pub enum Event {
    GridChanged,
}

pub struct SessionOptions {
    pub program: String,
    pub args: Vec<String>,
    pub cols: u16,
    pub rows: u16,
    pub cwd: Option<std::path::PathBuf>,
}

struct TermSize {
    cols: usize,
    rows: usize,
}

impl Dimensions for TermSize {
    fn total_lines(&self) -> usize {
        self.rows
    }
    fn screen_lines(&self) -> usize {
        self.rows
    }
    fn columns(&self) -> usize {
        self.cols
    }
}

enum Command {
    GetFrame(oneshot::Sender<Frame>),
    Resize(u16, u16, oneshot::Sender<io::Result<()>>),
    ContainsText(String, oneshot::Sender<bool>),
    MatchesRegex(Regex, oneshot::Sender<bool>),
    GetText(oneshot::Sender<String>),
}

/// A handle to a terminal session running in the background.
#[derive(Clone)]
pub struct Session {
    master_fd: Arc<OwnedFd>,
    child: Arc<Child>,
    // Channel to send commands to the background actor (e.g. state queries, resizes)
    tx_cmd: mpsc::Sender<Command>,
    // Channel that broadcasts simple notifications when the terminal grid updates
    tx_event: broadcast::Sender<Event>,
    // Channel that broadcasts fully rendered terminal frames (used for IPC streams to attached clients)
    tx_frame: broadcast::Sender<Frame>,
    // Channel that broadcasts the raw byte output stream straight from the PTY
    tx_output: broadcast::Sender<Vec<u8>>,
}

impl Session {
    /// Spawns the session process and starts the background task.
    /// Must be called from within a Tokio runtime.
    pub fn spawn(options: SessionOptions) -> io::Result<Self> {
        let pty = open_pty()?;
        let master_fd = pty.master;
        let slave = pty.slave;
        set_pty_size(slave.as_raw_fd(), options.cols, options.rows)?;

        let child = spawn_child(
            slave,
            &options.program,
            &options.args,
            options.cwd.as_deref(),
        )?;

        let term = Term::new(
            Config::default(),
            &TermSize {
                cols: options.cols as usize,
                rows: options.rows as usize,
            },
            VoidListener,
        );
        let parser = Processor::new();
        let (tx_event, _) = broadcast::channel(100);
        let (tx_frame, _) = broadcast::channel(100);
        let (tx_output, _) = broadcast::channel(100);
        let (tx_cmd, rx_cmd) = mpsc::channel(32);

        let master_fd_dup = master_fd.try_clone()?;
        let tx_event_clone = tx_event.clone();
        let tx_frame_clone = tx_frame.clone();
        let tx_output_clone = tx_output.clone();
        let master_fd_raw = master_fd.as_raw_fd();

        tokio::spawn(async move {
            run_actor(
                master_fd_dup,
                master_fd_raw,
                term,
                parser,
                rx_cmd,
                tx_event_clone,
                tx_frame_clone,
                tx_output_clone,
            )
            .await;
        });

        Ok(Self {
            master_fd: Arc::new(master_fd),
            child: Arc::new(child),
            tx_cmd,
            tx_event,
            tx_frame,
            tx_output,
        })
    }

    pub fn pid(&self) -> u32 {
        self.child.pid as u32
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.tx_event.subscribe()
    }

    pub fn subscribe_frame(&self) -> broadcast::Receiver<Frame> {
        self.tx_frame.subscribe()
    }

    pub fn subscribe_output(&self) -> broadcast::Receiver<Vec<u8>> {
        self.tx_output.subscribe()
    }

    pub async fn get_frame(&self) -> io::Result<Frame> {
        let (tx, rx) = oneshot::channel();
        if self.tx_cmd.send(Command::GetFrame(tx)).await.is_err() {
            return Err(io::Error::new(io::ErrorKind::Other, "session actor closed"));
        }
        rx.await.map_err(|_| {
            io::Error::new(io::ErrorKind::Other, "session actor dropped reply channel")
        })
    }

    pub async fn get_text(&self) -> io::Result<String> {
        let (tx, rx) = oneshot::channel();
        if self.tx_cmd.send(Command::GetText(tx)).await.is_err() {
            return Err(io::Error::new(io::ErrorKind::Other, "session actor closed"));
        }
        rx.await.map_err(|_| {
            io::Error::new(io::ErrorKind::Other, "session actor dropped reply channel")
        })
    }

    pub async fn resize(&self, cols: u16, rows: u16) -> io::Result<()> {
        let (tx, rx) = oneshot::channel();
        if self
            .tx_cmd
            .send(Command::Resize(cols, rows, tx))
            .await
            .is_err()
        {
            return Err(io::Error::new(io::ErrorKind::Other, "session actor closed"));
        }
        rx.await.map_err(|_| {
            io::Error::new(io::ErrorKind::Other, "session actor dropped reply channel")
        })?
    }

    pub fn write_input(&self, data: &[u8]) -> io::Result<()> {
        write_all_fd(self.master_fd.as_raw_fd(), data)
    }

    pub fn type_text(&self, text: &str) -> io::Result<()> {
        self.write_input(text.as_bytes())
    }

    pub fn input(&self) -> Input<'_> {
        Input { session: self }
    }

    pub async fn by_text(&self, text: &str) -> Result<(), String> {
        let mut rx = self.subscribe();

        loop {
            // Check current state
            let (tx, rx_cmd) = oneshot::channel();
            if self
                .tx_cmd
                .send(Command::ContainsText(text.to_string(), tx))
                .await
                .is_ok()
            {
                if rx_cmd.await.unwrap_or(false) {
                    return Ok(());
                }
            } else {
                return Err("session closed".into());
            }

            // Wait for next change
            while let Ok(event) = rx.recv().await {
                if matches!(event, Event::GridChanged) {
                    let (tx, rx_cmd) = oneshot::channel();
                    if self
                        .tx_cmd
                        .send(Command::ContainsText(text.to_string(), tx))
                        .await
                        .is_ok()
                    {
                        if rx_cmd.await.unwrap_or(false) {
                            return Ok(());
                        }
                    } else {
                        return Err("session closed".into());
                    }
                }
            }
        }
    }

    pub async fn by_regex(&self, pattern: &str) -> Result<(), String> {
        let regex = Regex::new(pattern).map_err(|e| e.to_string())?;
        let mut rx = self.subscribe();

        loop {
            // Check current state
            let (tx, rx_cmd) = oneshot::channel();
            if self
                .tx_cmd
                .send(Command::MatchesRegex(regex.clone(), tx))
                .await
                .is_ok()
            {
                if rx_cmd.await.unwrap_or(false) {
                    return Ok(());
                }
            } else {
                return Err("session closed".into());
            }

            // Wait for next change
            while let Ok(event) = rx.recv().await {
                if matches!(event, Event::GridChanged) {
                    let (tx, rx_cmd) = oneshot::channel();
                    if self
                        .tx_cmd
                        .send(Command::MatchesRegex(regex.clone(), tx))
                        .await
                        .is_ok()
                    {
                        if rx_cmd.await.unwrap_or(false) {
                            return Ok(());
                        }
                    } else {
                        return Err("session closed".into());
                    }
                }
            }
        }
    }

    pub async fn wait_for_activity(&self) -> io::Result<()> {
        let mut rx = self.subscribe();

        // 1. Wait for the FIRST change
        loop {
            match rx.recv().await {
                Ok(Event::GridChanged) => break,
                Err(_) => return Err(io::Error::new(io::ErrorKind::Other, "session actor closed")),
            }
        }

        // 2. Wait for quiescence (no changes for 2/60s)
        let settle_duration = Duration::from_nanos(1_000_000_000 * 2 / 60);
        loop {
            tokio::select! {
                res = rx.recv() => {
                    match res {
                        Ok(Event::GridChanged) => continue,
                        Err(_) => return Ok(()),
                    }
                }
                _ = tokio::time::sleep(settle_duration) => {
                    return Ok(());
                }
            }
        }
    }

    pub async fn wait_for_idle(&self) -> io::Result<()> {
        let mut rx = self.subscribe();
        let settle_duration = Duration::from_nanos(1_000_000_000 * 2 / 60);
        loop {
            tokio::select! {
                res = rx.recv() => {
                    match res {
                        Ok(Event::GridChanged) => continue,
                        Err(_) => return Ok(()),
                    }
                }
                _ = tokio::time::sleep(settle_duration) => {
                    return Ok(());
                }
            }
        }
    }

    /// Starts a background IPC server on a Unix socket, allowing
    /// `claco-termulator attach` to connect to this session.
    pub async fn serve_ipc(&self) -> io::Result<()> {
        use crate::ipc::socket_path;

        let path = socket_path(self.pid());
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path)?;
        let tx_frame = self.tx_frame.clone();

        tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    break;
                };
                let mut sub = tx_frame.subscribe();

                tokio::spawn(async move {
                    let framed = LengthDelimitedCodec::builder().new_framed(stream);
                    let mut sink: tokio_serde::Framed<_, Frame, Frame, Bincode<Frame, Frame>> =
                        tokio_serde::Framed::new(framed, Bincode::default());

                    while let Ok(frame) = sub.recv().await {
                        if sink.send(frame).await.is_err() {
                            break;
                        }
                    }
                });
            }
        });

        Ok(())
    }
}

async fn run_actor(
    master_fd_dup: OwnedFd,
    master_fd_raw: i32,
    mut term: Term<VoidListener>,
    mut parser: Processor,
    mut rx_cmd: mpsc::Receiver<Command>,
    tx_event: broadcast::Sender<Event>,
    tx_frame: broadcast::Sender<Frame>,
    tx_output: broadcast::Sender<Vec<u8>>,
) {
    let mut reader = tokio::fs::File::from_std(unsafe {
        std::fs::File::from_raw_fd(master_fd_dup.into_raw_fd())
    });
    let mut buf = [0u8; 4096];

    loop {
        tokio::select! {
            result = reader.read(&mut buf) => {
                match result {
                    Ok(0) => break,
                    Ok(n) => {
                        let data = &buf[..n];
                        parser.advance(&mut term, data);
                        let _ = tx_event.send(Event::GridChanged);
                        if tx_frame.receiver_count() > 0 {
                            let _ = tx_frame.send(term_to_frame(&term));
                        }
                        let _ = tx_output.send(data.to_vec());
                    }
                    Err(_) => break,
                }
            }
            cmd = rx_cmd.recv() => {
                match cmd {
                    Some(Command::GetFrame(reply)) => {
                        let _ = reply.send(term_to_frame(&term));
                    }
                    Some(Command::Resize(cols, rows, reply)) => {
                        let res = set_pty_size(master_fd_raw, cols, rows);
                        if res.is_ok() {
                            term.resize(TermSize {
                                cols: cols as usize,
                                rows: rows as usize,
                            });
                        }
                        let _ = reply.send(res);
                    }
                    Some(Command::ContainsText(text, reply)) => {
                        let _ = reply.send(term_contains_text(&term, &text));
                    }
                    Some(Command::MatchesRegex(regex, reply)) => {
                        let _ = reply.send(term_matches_regex(&term, &regex));
                    }
                    Some(Command::GetText(reply)) => {
                        let _ = reply.send(term_get_text(&term));
                    }
                    None => break,
                }
            }
        }
    }
}

fn term_get_text(term: &Term<VoidListener>) -> String {
    let mut full_text = String::new();
    let cols = term.columns();
    for indexed in term.grid().display_iter() {
        full_text.push(indexed.cell.c);
        if indexed.point.column.0 == cols - 1 {
            full_text.push('\n');
        }
    }
    full_text
}

fn term_contains_text(term: &Term<VoidListener>, text: &str) -> bool {
    term_get_text(term).contains(text)
}

fn term_matches_regex(term: &Term<VoidListener>, regex: &Regex) -> bool {
    regex.is_match(&term_get_text(term))
}

fn write_all_fd(fd: i32, data: &[u8]) -> io::Result<()> {
    let mut written = 0;
    while written < data.len() {
        let ret = unsafe {
            libc::write(
                fd,
                data[written..].as_ptr() as *const libc::c_void,
                data.len() - written,
            )
        };
        if ret < 0 {
            let e = io::Error::last_os_error();
            if matches!(e.raw_os_error(), Some(libc::EIO) | Some(libc::ENXIO)) {
                return Ok(());
            }
            return Err(e);
        }
        written += ret as usize;
    }
    Ok(())
}

pub struct Input<'a> {
    session: &'a Session,
}

impl<'a> Input<'a> {
    pub fn text(self, text: &str) -> Self {
        let _ = self.session.type_text(text);
        self
    }

    pub fn enter(self) -> Self {
        let _ = self.session.write_input(b"\x1b[13u");
        self
    }

    pub fn tab(self) -> Self {
        let _ = self.session.write_input(b"\x1b[9u");
        self
    }

    pub fn up(self) -> Self {
        let _ = self.session.write_input(b"\x1b[1;A");
        self
    }

    pub fn down(self) -> Self {
        let _ = self.session.write_input(b"\x1b[1;B");
        self
    }

    pub fn right(self) -> Self {
        let _ = self.session.write_input(b"\x1b[1;C");
        self
    }

    pub fn left(self) -> Self {
        let _ = self.session.write_input(b"\x1b[1;D");
        self
    }

    pub fn backspace(self) -> Self {
        let _ = self.session.write_input(b"\x1b[127u");
        self
    }

    pub fn del(self) -> Self {
        let _ = self.session.write_input(b"\x1b[3~");
        self
    }

    pub fn esc(self) -> Self {
        let _ = self.session.write_input(b"\x1b[27u");
        self
    }

    pub fn home(self) -> Self {
        let _ = self.session.write_input(b"\x1b[1;H");
        self
    }

    pub fn end(self) -> Self {
        let _ = self.session.write_input(b"\x1b[1;F");
        self
    }

    pub fn pgup(self) -> Self {
        let _ = self.session.write_input(b"\x1b[5~");
        self
    }

    pub fn pgdn(self) -> Self {
        let _ = self.session.write_input(b"\x1b[6~");
        self
    }

    pub fn space(self) -> Self {
        let _ = self.session.write_input(b" ");
        self
    }

    pub fn ctrl(self, c: char) -> Self {
        if c.is_ascii_lowercase() {
            let b = (c as u8) - b'a' + 1;
            let _ = self.session.write_input(&[b]);
        } else if c.is_ascii_uppercase() {
            let b = (c as u8) - b'A' + 1;
            let _ = self.session.write_input(&[b]);
        }
        self
    }
}
