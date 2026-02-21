use std::{
    ffi::CString,
    io,
    os::unix::io::{AsRawFd, OwnedFd, RawFd},
};

use libc::{
    STDERR_FILENO, STDIN_FILENO, STDOUT_FILENO, TIOCSCTTY, close, dup2, execvp, fork, setsid,
};

pub struct Child {
    pub pid: libc::pid_t,
}

fn setup_slave_stdio(slave_fd: RawFd) -> io::Result<()> {
    for target in [STDIN_FILENO, STDOUT_FILENO, STDERR_FILENO] {
        if unsafe { dup2(slave_fd, target) } < 0 {
            return Err(io::Error::last_os_error());
        }
    }
    // Close the original slave fd if it wasn't one of the stdio fds.
    if slave_fd > STDERR_FILENO {
        unsafe { close(slave_fd) };
    }
    Ok(())
}

fn exec_process(program: &str, args: &[String]) -> io::Result<()> {
    let program_c = CString::new(program)?;
    let argv: Vec<CString> = std::iter::once(CString::new(program)?)
        .chain(args.iter().map(|a| CString::new(a.as_str()).unwrap()))
        .collect();
    let mut argv_ptrs: Vec<*const libc::c_char> = argv.iter().map(|s| s.as_ptr()).collect();
    argv_ptrs.push(std::ptr::null());

    unsafe { execvp(program_c.as_ptr(), argv_ptrs.as_ptr()) };
    // execvp only returns on error.
    Err(io::Error::last_os_error())
}

pub fn spawn_child(
    slave: OwnedFd,
    program: &str,
    args: &[String],
    cwd: Option<&std::path::Path>,
) -> io::Result<Child> {
    let slave_fd = slave.as_raw_fd();

    let pid = unsafe { fork() };
    match pid {
        -1 => Err(io::Error::last_os_error()),

        // Child process: become session leader, attach TTY, exec.
        0 => {
            // Do not drop the FD, otherwise it gets closed before ioctl!
            // We manually close it in setup_slave_stdio.
            std::mem::forget(slave);

            if unsafe { setsid() } < 0 {
                eprintln!("setsid failed: {}", io::Error::last_os_error());
                std::process::exit(1);
            }

            if unsafe { libc::ioctl(slave_fd, TIOCSCTTY as _, 0) } < 0 {
                eprintln!("TIOCSCTTY failed: {}", io::Error::last_os_error());
                std::process::exit(1);
            }

            if let Err(e) = setup_slave_stdio(slave_fd) {
                eprintln!("stdio setup failed: {e}");
                std::process::exit(1);
            }

            if let Some(path) = cwd {
                if let Err(e) = std::env::set_current_dir(path) {
                    eprintln!("chdir failed: {e}");
                    std::process::exit(1);
                }
            }

            if let Err(e) = exec_process(program, args) {
                eprintln!("exec failed: {e}");
                std::process::exit(1);
            }

            unreachable!()
        }

        // Parent: slave fd is owned by child; drop our copy.
        pid => {
            drop(slave);
            Ok(Child { pid })
        }
    }
}

pub fn wait_child(child: &Child) -> io::Result<i32> {
    let mut status: libc::c_int = 0;
    let ret = unsafe { libc::waitpid(child.pid, &mut status, 0) };
    if ret < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(libc::WEXITSTATUS(status))
}
