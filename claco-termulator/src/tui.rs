use ratatui::{
    Frame as RFrame,
    layout::Rect,
    style::{Color as RColor, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

use crate::ipc::{Color, Frame};

pub fn ui(f: &mut RFrame, frame: &Frame, mouse_pos: Option<(u16, u16)>) {
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

pub struct TermWidget<'a>(pub &'a Frame);

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

pub fn color_to_ratatui(color: Color) -> RColor {
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
            // Indices >= 256 (like 256 for FG, 257 for BG) map to Reset (transparent/default)
            _ => RColor::Reset,
        },
        Color::Indexed(i) => RColor::Indexed(i),
        Color::Rgb(r, g, b) => RColor::Rgb(r, g, b),
    }
}
