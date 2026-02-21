//! claco-termulator library for PTY management and terminal emulation.

pub mod attach;
pub mod frame_conv;
pub mod ipc;
pub mod run;
pub mod session;
pub mod tui;
pub mod view;

/// A single cell in the terminal grid.
pub use ipc::{Cell, Color, Frame};
/// A terminal session manager.
pub use session::{Event, Session, SessionOptions};
