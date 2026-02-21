mod record;

use std::{os::unix::io::AsRawFd, path::PathBuf, process as std_process};

use claco_pty::{
    process::{spawn_child, wait_child},
    pty::{
        configure_raw_termios, get_terminal_size, open_pty, open_slave, restore_termios,
        set_pty_size,
    },
};
use clap::Parser;
use record::{Recorder, record_session};

#[derive(Parser)]
#[command(name = "claco-rec", about = "Record a terminal session to a raw file")]
struct Args {
    #[arg(
        short,
        long,
        default_value = "recording.raw",
        help = "Output file path"
    )]
    output: PathBuf,

    #[arg(
        long,
        help = "Terminal width (columns); defaults to current terminal size"
    )]
    cols: Option<u16>,

    #[arg(
        long,
        help = "Terminal height (rows); defaults to current terminal size"
    )]
    rows: Option<u16>,

    #[arg(required = true, help = "Program to record")]
    program: String,

    #[arg(trailing_var_arg = true, help = "Arguments for the program")]
    prog_args: Vec<String>,
}

fn run() -> std::io::Result<i32> {
    let args = Args::parse();

    let (cols, rows) = match (args.cols, args.rows) {
        (Some(c), Some(r)) => (c, r),
        _ => get_terminal_size().unwrap_or((220, 50)),
    };

    let pty = open_pty()?;
    let slave = open_slave(&pty.slave_path)?;
    set_pty_size(slave.as_raw_fd(), cols, rows)?;

    let child = spawn_child(slave, &args.program, &args.prog_args, None)?;
    let mut recorder = Recorder::open(&args.output)?;

    let stdin_orig = configure_raw_termios(libc::STDIN_FILENO).ok();
    record_session(&pty.master, &mut recorder)?;
    if let Some(ref orig) = stdin_orig {
        restore_termios(libc::STDIN_FILENO, orig);
    }

    wait_child(&child)
}

fn main() {
    match run() {
        Ok(code) => std_process::exit(code),
        Err(e) => {
            eprintln!("claco-rec: {e}");
            std_process::exit(1);
        }
    }
}
