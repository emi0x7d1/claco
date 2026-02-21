use std::process as std_process;

use claco_termulator::{attach, run, view};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "claco-termulator",
    about = "Terminal multiplexer with broadcast attach"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Spawn a program under a PTY and broadcast its terminal state.
    Run(run::RunArgs),
    /// Attach to a running session as a read-only mirror.
    Attach(attach::AttachArgs),
    /// View a serialized Frame JSON file.
    View(view::ViewArgs),
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Run(args) => {
            let code = run::run(&args).unwrap_or_else(|e| {
                eprintln!("claco-termulator run: {e}");
                std_process::exit(1);
            });
            std_process::exit(code);
        }
        Command::Attach(args) => {
            attach::attach(&args).unwrap_or_else(|e| {
                eprintln!("claco-termulator attach: {e}");
                std_process::exit(1);
            });
        }
        Command::View(args) => {
            view::view(&args).unwrap_or_else(|e| {
                eprintln!("claco-termulator view: {e}");
                std_process::exit(1);
            });
        }
    }
}
