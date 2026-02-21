use crossterm::{
    event::{self, Event, EventStream, KeyCode, MouseEvent, MouseEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use futures::StreamExt;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{fs, io};

use crate::ipc::Frame;
use crate::tui::ui;

#[derive(Clone, clap::Args)]
pub struct ViewArgs {
    /// Path to the serialized Frame JSON file.
    pub path: String,
}

pub fn view(args: &ViewArgs) -> io::Result<()> {
    let content = fs::read_to_string(&args.path)?;
    let frame: Frame = serde_json::from_str(&content).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("failed to parse frame: {e}"),
        )
    })?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, event::EnableMouseCapture)?;
    use std::io::Write;
    let _ = write!(stdout, "\x1b[?1003h");
    let _ = stdout.flush();

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    let result = rt.block_on(view_async(frame, &mut terminal));

    disable_raw_mode()?;
    let mut stdout2 = io::stdout();
    let _ = write!(stdout2, "\x1b[?1003l");
    let _ = stdout2.flush();
    execute!(stdout2, LeaveAlternateScreen, event::DisableMouseCapture)?;
    terminal.show_cursor()?;

    result
}

async fn view_async(
    frame: Frame,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> io::Result<()> {
    let mut events = EventStream::new();
    let mut mouse_pos: Option<(u16, u16)> = None;

    loop {
        terminal.draw(|f| {
            ui(f, &frame, mouse_pos);
        })?;

        tokio::select! {
            ev = events.next() => {
                match ev {
                    Some(Ok(Event::Key(key))) => {
                        if key.code == KeyCode::Char('q') || key.code == KeyCode::Esc ||
                           (key.code == KeyCode::Char('c') && key.modifiers.contains(event::KeyModifiers::CONTROL)) {
                            return Ok(());
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
