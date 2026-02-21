use serde::{Deserialize, Serialize};

/// A terminal color, kept as the original ANSI representation so that the
/// attaching terminal's own palette is used for named and indexed colors.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Color {
    Named(u16),
    Indexed(u8),
    Rgb(u8, u8, u8),
}

/// Cell attributes packed as bitflags: bold=1, italic=2, underline=4, reverse=8.
pub type Flags = u8;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Cell {
    pub ch: char,
    pub fg: Color,
    pub bg: Color,
    pub flags: Flags,
}

/// A complete rendered grid snapshot broadcast from `run` to all `attach` clients.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Frame {
    pub cols: u16,
    pub rows: u16,
    pub cursor_x: u16,
    pub cursor_y: u16,
    pub cells: Vec<Cell>,
}

pub fn socket_path(pid: u32) -> String {
    format!("/tmp/claco-termulator-{pid}.sock")
}
