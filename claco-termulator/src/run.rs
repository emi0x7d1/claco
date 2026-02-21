use std::io;

use crate::{
    ipc::socket_path,
    session::{Session, SessionOptions},
};

#[derive(Clone, clap::Args)]
pub struct RunArgs {
    pub program: String,
    pub args: Vec<String>,
}

use claco_pty::pty::{configure_raw_termios, get_terminal_size, restore_termios};

pub fn run(run_args: &RunArgs) -> io::Result<i32> {
    let (cols, rows) = get_terminal_size().unwrap_or((220, 50));

    // Create the tokio runtime for async I/O.
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let _guard = rt.enter();

    let session = Session::spawn(SessionOptions {
        program: run_args.program.clone(),
        args: run_args.args.clone(),
        cols,
        rows,
        cwd: None,
    })?;

    // Start IPC server for broadcast
    rt.block_on(session.serve_ipc())?;

    let stdin_orig = configure_raw_termios(libc::STDIN_FILENO).ok();
    let result = rt.block_on(relay_loop(&session));
    if let Some(ref orig) = stdin_orig {
        restore_termios(libc::STDIN_FILENO, orig);
    }
    let _ = std::fs::remove_file(&socket_path(session.pid()));

    result?;
    // For now we don't have wait_child in Session easily, so we might need a way to wait for it.
    // However, if the process exited, Session will stop reading.
    // Let's just return 0 for now or implement a wait method.
    Ok(0)
}

async fn relay_loop(session: &Session) -> io::Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let mut stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut rx_output = session.subscribe_output();
    let mut buf = [0u8; 4096];

    loop {
        tokio::select! {
            // Data from child → stdout
            result = rx_output.recv() => {
                match result {
                    Ok(data) => {
                        stdout.write_all(&data).await?;
                        stdout.flush().await?;
                    }
                    Err(_) => break, // Session output closed
                }
            }
            // Keystroke from user stdin → child
            result = stdin.read(&mut buf) => {
                match result {
                    Ok(0) => break,
                    Ok(n) => {
                        session.write_input(&buf[..n])?;
                    }
                    Err(e) => return Err(e),
                }
            }
        }
    }

    Ok(())
}
