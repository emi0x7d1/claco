use futures::StreamExt;
use std::{fs, io};
use tokio::net::UnixStream;
use tokio_serde::formats::Bincode;
use tokio_util::codec::LengthDelimitedCodec;

use crossterm::{
    event::{self, Event, EventStream, KeyCode, MouseEvent, MouseEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame as RFrame, Terminal,
    backend::CrosstermBackend,
    layout::Rect,
    style::{Color as RColor, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

use crate::ipc::{Color, Frame, socket_path};

#[derive(Clone, clap::Args)]
pub struct AttachArgs {
    /// PID of the session to attach to (auto-detected if omitted).
    pub pid: Option<u32>,
}

pub fn attach(args: &AttachArgs) -> io::Result<()> {
    let mut sock_path = resolve_socket(args.pid);
    if let Err(ref e) = sock_path {
        if e.kind() == io::ErrorKind::NotFound {
            eprintln!("Waiting for claco-termulator session...");
            while let Err(ref e) = sock_path {
                if e.kind() != io::ErrorKind::NotFound {
                    return Err(sock_path.expect_err("was error"));
                }
                std::thread::sleep(std::time::Duration::from_millis(250));
                sock_path = resolve_socket(args.pid);
            }
        }
    }
    let sock_path = sock_path?;

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, event::EnableMouseCapture)?;
    use std::io::Write;
    let _ = write!(stdout, "\x1b[?1003h");
    let _ = stdout.flush();

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let result = rt.block_on(attach_async(sock_path, &mut terminal));

    disable_raw_mode()?;
    let mut stdout2 = io::stdout();
    let _ = write!(stdout2, "\x1b[?1003l");
    let _ = stdout2.flush();
    execute!(stdout2, LeaveAlternateScreen, event::DisableMouseCapture)?;
    terminal.show_cursor()?;

    result
}

async fn attach_async(
    sock_path: String,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> io::Result<()> {
    let stream = UnixStream::connect(&sock_path)
        .await
        .map_err(|e| io::Error::new(e.kind(), format!("cannot connect to {sock_path}: {e}")))?;

    let framed = LengthDelimitedCodec::builder().new_framed(stream);
    let mut source: tokio_serde::Framed<_, Frame, Frame, Bincode<Frame, Frame>> =
        tokio_serde::Framed::new(framed, Bincode::default());

    let mut clipboard = arboard::Clipboard::new().ok();
    let mut events = EventStream::new();
    let mut last_frame: Option<Frame> = None;
    let mut mouse_pos: Option<(u16, u16)> = None;

    loop {
        if let Some(ref frame) = last_frame {
            terminal.draw(|f| {
                ui(f, frame, mouse_pos);
            })?;
        }

        tokio::select! {
            msg = source.next() => {
                match msg {
                    Some(Ok(frame)) => {
                        last_frame = Some(frame);
                    }
                    Some(Err(e)) if e.kind() == io::ErrorKind::UnexpectedEof => {
                        eprintln!("\r\n[claco-termulator: session ended]");
                        return Ok(());
                    }
                    Some(Err(e)) => return Err(e),
                    None => {
                        eprintln!("\r\n[claco-termulator: session ended]");
                        return Ok(());
                    }
                }
            }
            ev = events.next() => {
                match ev {
                    Some(Ok(Event::Key(key))) => {
                        if key.code == KeyCode::Char('q') || key.code == KeyCode::Esc ||
                           (key.code == KeyCode::Char('c') && key.modifiers.contains(event::KeyModifiers::CONTROL)) {
                            return Ok(());
                        }

                        if key.code == KeyCode::Char('c') && key.modifiers.is_empty() {
                            if let Some(ref frame) = last_frame {
                                if let Some(ref mut cb) = clipboard {
                                    if let Ok(json) = serde_json::to_string(frame) {
                                        let _ = cb.set_text(json);
                                    }
                                }
                            }
                        }
                    }
                    Some(Ok(Event::Mouse(MouseEvent { kind, column, row, .. }))) => {
                        if kind == MouseEventKind::Moved || matches!(kind, MouseEventKind::Drag(_)) {
                            mouse_pos = Some((column, row));
                        }
                    }
                    Some(Err(e)) => return Err(e),
                    None => return Ok(()),
                    _ => {}
                }
            }
        }
    }
}

fn ui(f: &mut RFrame, frame: &Frame, mouse_pos: Option<(u16, u16)>) {
    let size = f.size();
    f.render_widget(TermWidget(frame), size);

    if frame.cursor_x < size.width && frame.cursor_y < size.height {
        f.set_cursor(frame.cursor_x, frame.cursor_y);
    }

    if let Some(pos) = mouse_pos {
        if pos.0 < frame.cols && pos.1 < frame.rows {
            let idx = pos.1 as usize * frame.cols as usize + pos.0 as usize;
            if let Some(cell) = frame.cells.get(idx) {
                let tooltip_lines = vec![
                    Line::from(format!(
                        "Char: '{}' (U+{:04X})",
                        cell.ch.escape_default(),
                        cell.ch as u32
                    )),
                    Line::from(format!("FG: {:?}", cell.fg)),
                    Line::from(format!("BG: {:?}", cell.bg)),
                    Line::from(format!("Flags: 0x{:02X}", cell.flags)),
                    Line::from(format!(
                        "Global Cursor: ({}, {})",
                        frame.cursor_x, frame.cursor_y
                    )),
                ];

                let tooltip_width = tooltip_lines
                    .iter()
                    .map(|l| l.width() as u16)
                    .max()
                    .unwrap_or(0)
                    + 2;
                let tooltip_height = tooltip_lines.len() as u16 + 2;

                let mut tooltip_x = pos.0 + 1;
                let mut tooltip_y = pos.1 + 1;

                if tooltip_x + tooltip_width > size.width {
                    tooltip_x = pos.0.saturating_sub(tooltip_width);
                }
                if tooltip_y + tooltip_height > size.height {
                    tooltip_y = pos.1.saturating_sub(tooltip_height);
                }

                let tooltip_area = Rect::new(tooltip_x, tooltip_y, tooltip_width, tooltip_height);
                let tooltip = Paragraph::new(tooltip_lines)
                    .block(Block::default().borders(Borders::ALL).title("Properties"))
                    .style(Style::default().bg(RColor::DarkGray).fg(RColor::White));

                f.render_widget(Clear, tooltip_area);
                f.render_widget(tooltip, tooltip_area);
            }
        }
    }
}

struct TermWidget<'a>(&'a Frame);

impl<'a> Widget for TermWidget<'a> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let frame = self.0;
        let mut row = 0;
        let mut col = 0;

        for cell in &frame.cells {
            if col < area.width && row < area.height {
                let cell_x = area.x + col;
                let cell_y = area.y + row;
                let buf_cell = buf.get_mut(cell_x, cell_y);

                let style = Style::default()
                    .fg(color_to_ratatui(cell.fg))
                    .bg(color_to_ratatui(cell.bg));

                let mut modifier = Modifier::empty();
                if cell.flags & 0x01 != 0 {
                    modifier.insert(Modifier::BOLD);
                }
                if cell.flags & 0x02 != 0 {
                    modifier.insert(Modifier::ITALIC);
                }
                if cell.flags & 0x04 != 0 {
                    modifier.insert(Modifier::UNDERLINED);
                }
                if cell.flags & 0x08 != 0 {
                    modifier.insert(Modifier::REVERSED);
                }

                buf_cell.set_symbol(&cell.ch.to_string());
                buf_cell.set_style(style.add_modifier(modifier));
            }

            col += 1;
            if col == frame.cols {
                col = 0;
                row += 1;
            }
        }
    }
}

fn color_to_ratatui(color: Color) -> RColor {
    match color {
        Color::Named(n) => match n {
            0 => RColor::Black,
            1 => RColor::Red,
            2 => RColor::Green,
            3 => RColor::Yellow,
            4 => RColor::Blue,
            5 => RColor::Magenta,
            6 => RColor::Cyan,
            7 => RColor::Gray,
            8 => RColor::DarkGray,
            9 => RColor::LightRed,
            10 => RColor::LightGreen,
            11 => RColor::LightYellow,
            12 => RColor::LightBlue,
            13 => RColor::LightMagenta,
            14 => RColor::LightCyan,
            15 => RColor::White,
            _ => RColor::Reset,
        },
        Color::Indexed(i) => RColor::Indexed(i),
        Color::Rgb(r, g, b) => RColor::Rgb(r, g, b),
    }
}

fn resolve_socket(pid: Option<u32>) -> io::Result<String> {
    if let Some(pid) = pid {
        let path = socket_path(pid);
        if std::path::Path::new(&path).exists() {
            return Ok(path);
        }
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("no running session for pid {pid}"),
        ));
    }

    let mut found: Vec<String> = fs::read_dir("/tmp")?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("claco-termulator-") && n.ends_with(".sock"))
        .map(|n| format!("/tmp/{n}"))
        .collect();

    match found.len() {
        0 => Err(io::Error::new(
            io::ErrorKind::NotFound,
            "no running claco-termulator sessions found",
        )),
        1 => Ok(found.remove(0)),
        _ => {
            eprintln!("Multiple sessions found:");
            for (i, p) in found.iter().enumerate() {
                let pid_str = p
                    .trim_start_matches("/tmp/claco-termulator-")
                    .trim_end_matches(".sock");
                eprintln!("  [{i}] PID {pid_str}");
            }
            eprintln!("Run: claco-termulator attach <PID>");
            Err(io::Error::new(
                io::ErrorKind::Other,
                "ambiguous — specify a PID",
            ))
        }
    }
}
