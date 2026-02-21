use std::{
    fs::{File, OpenOptions},
    io::{self, Write},
    os::unix::io::{AsRawFd, OwnedFd, RawFd},
    path::Path,
};

use libc::{POLLIN, nfds_t, poll, pollfd};

const BUF_SIZE: usize = 4096;

pub struct Recorder {
    output: File,
}

impl Recorder {
    pub fn open(path: &Path) -> io::Result<Self> {
        let output = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;
        Ok(Self { output })
    }
}

fn poll_readable(fds: &mut [pollfd]) -> io::Result<i32> {
    let ret = unsafe { poll(fds.as_mut_ptr(), fds.len() as nfds_t, -1) };
    if ret < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(ret)
    }
}

fn read_available(fd: RawFd, buf: &mut [u8]) -> io::Result<usize> {
    let ret = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
    if ret < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(ret as usize)
    }
}

fn write_all_raw(fd: RawFd, data: &[u8]) -> io::Result<()> {
    let mut written = 0;
    while written < data.len() {
        let ret = unsafe {
            libc::write(
                fd,
                data[written..].as_ptr() as *const libc::c_void,
                data.len() - written,
            )
        };
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }
        written += ret as usize;
    }
    Ok(())
}

// Relay data between stdin/stdout and the master PTY while also writing
// everything coming from the master to the recording file.
pub fn record_session(master: &OwnedFd, recorder: &mut Recorder) -> io::Result<()> {
    let master_fd = master.as_raw_fd();
    let stdin_fd = libc::STDIN_FILENO;
    let stdout_fd = libc::STDOUT_FILENO;
    let mut buf = vec![0u8; BUF_SIZE];

    loop {
        let mut fds = [
            pollfd {
                fd: master_fd,
                events: POLLIN,
                revents: 0,
            },
            pollfd {
                fd: stdin_fd,
                events: POLLIN,
                revents: 0,
            },
        ];

        if poll_readable(&mut fds)? == 0 {
            continue;
        }

        // Data from child (PTY master) → stdout + recording file.
        if fds[0].revents & POLLIN != 0 {
            match read_available(master_fd, &mut buf) {
                Ok(0) => break,
                Err(e) if is_eof_error(&e) => break,
                Err(e) => return Err(e),
                Ok(n) => {
                    let data = &buf[..n];
                    write_all_raw(stdout_fd, data)?;
                    recorder.output.write_all(data)?;
                }
            }
        }

        // Keystrokes from user stdin → PTY master (forwarded to child).
        if fds[1].revents & POLLIN != 0 {
            match read_available(stdin_fd, &mut buf) {
                Ok(0) => break,
                Err(e) if is_eof_error(&e) => break,
                Err(e) => return Err(e),
                Ok(n) => write_all_raw(master_fd, &buf[..n])?,
            }
        }
    }

    Ok(())
}

fn is_eof_error(e: &io::Error) -> bool {
    matches!(e.raw_os_error(), Some(libc::EIO) | Some(libc::ENXIO))
}
