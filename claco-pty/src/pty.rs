use std::{
    ffi::CStr,
    io,
    os::unix::io::{AsRawFd, FromRawFd, OwnedFd, RawFd},
};

use libc::{O_NOCTTY, O_RDWR, TIOCGWINSZ, TIOCSWINSZ, termios, winsize};

pub struct Pty {
    pub master: OwnedFd,
    pub slave: OwnedFd,
    pub slave_path: String,
}

pub fn open_pty() -> io::Result<Pty> {
    let mut master: libc::c_int = 0;
    let mut slave: libc::c_int = 0;

    let ret = unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };

    if ret < 0 {
        return Err(io::Error::last_os_error());
    }

    let master = unsafe { OwnedFd::from_raw_fd(master) };
    let slave = unsafe { OwnedFd::from_raw_fd(slave) };

    let mut path_buf = [0; 1024];
    let ret = unsafe { libc::ttyname_r(slave.as_raw_fd(), path_buf.as_mut_ptr(), path_buf.len()) };
    if ret != 0 {
        return Err(io::Error::from_raw_os_error(ret));
    }
    let path = unsafe { CStr::from_ptr(path_buf.as_ptr()) }
        .to_string_lossy()
        .into_owned();

    Ok(Pty {
        master,
        slave,
        slave_path: path,
    })
}

pub fn open_slave(slave_path: &str) -> io::Result<OwnedFd> {
    let path = std::ffi::CString::new(slave_path)?;
    let fd = unsafe { libc::open(path.as_ptr(), O_RDWR | O_NOCTTY) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}

pub fn set_pty_size(fd: RawFd, cols: u16, rows: u16) -> io::Result<()> {
    let ws = winsize {
        ws_row: rows,
        ws_col: cols,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    let ret = unsafe { libc::ioctl(fd, TIOCSWINSZ as _, &ws as *const winsize) };
    if ret < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

pub fn get_terminal_size() -> Option<(u16, u16)> {
    let mut ws: winsize = unsafe { std::mem::zeroed() };
    let ret = unsafe {
        libc::ioctl(
            libc::STDOUT_FILENO,
            TIOCGWINSZ as _,
            &mut ws as *mut winsize,
        )
    };
    if ret == 0 && ws.ws_col > 0 && ws.ws_row > 0 {
        Some((ws.ws_col, ws.ws_row))
    } else {
        None
    }
}

pub fn configure_raw_termios(fd: RawFd) -> io::Result<termios> {
    let mut orig: termios = unsafe { std::mem::zeroed() };
    if unsafe { libc::tcgetattr(fd, &mut orig) } < 0 {
        return Err(io::Error::last_os_error());
    }
    let mut raw = orig;
    unsafe { libc::cfmakeraw(&mut raw) };
    if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &raw) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(orig)
}

pub fn restore_termios(fd: RawFd, orig: &termios) {
    unsafe { libc::tcsetattr(fd, libc::TCSANOW, orig) };
}
