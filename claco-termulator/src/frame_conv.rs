use alacritty_terminal::{Term, event::VoidListener, grid::Dimensions, term::cell::Flags};

use crate::ipc::{Cell, Color, Frame};

pub fn term_to_frame(term: &Term<VoidListener>) -> Frame {
    let cols = term.columns();
    let rows = term.screen_lines();
    let cursor = term.grid().cursor.point;
    let mut cells = Vec::with_capacity(cols * rows);

    for indexed in term.grid().display_iter() {
        let cell = &indexed.cell;
        let ch = if cell.c == '\0' { ' ' } else { cell.c };

        let mut flags: u8 = 0;
        if cell.flags.contains(Flags::BOLD) {
            flags |= 0x01;
        }
        if cell.flags.contains(Flags::ITALIC) {
            flags |= 0x02;
        }
        if cell.flags.contains(Flags::UNDERLINE) {
            flags |= 0x04;
        }
        if cell.flags.contains(Flags::INVERSE) {
            flags |= 0x08;
        }

        cells.push(Cell {
            ch,
            fg: ansi_to_ipc_color(cell.fg),
            bg: ansi_to_ipc_color(cell.bg),
            flags,
        });
    }

    Frame {
        cols: cols as u16,
        rows: rows as u16,
        cursor_x: cursor.column.0 as u16,
        cursor_y: cursor.line.0 as u16,
        cells,
    }
}

fn ansi_to_ipc_color(color: alacritty_terminal::vte::ansi::Color) -> Color {
    use alacritty_terminal::vte::ansi::Color as AnsiColor;
    match color {
        AnsiColor::Spec(rgb) => Color::Rgb(rgb.r, rgb.g, rgb.b),
        AnsiColor::Named(n) => Color::Named(n as u16),
        AnsiColor::Indexed(idx) => Color::Indexed(idx),
    }
}
