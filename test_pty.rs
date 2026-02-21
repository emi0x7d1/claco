use libc::{winsize, O_NOCTTY, O_RDWR, TIOCSWINSZ};
use std::os::unix::io::{AsRawFd, FromRawFd, OwnedFd, RawFd};

fn main() {
    let fd = unsafe { libc::posix_openpt(O_RDWR | O_NOCTTY) };
    println!("posix_openpt fd: {}", fd);
    if fd < 0 {
        println!("Error: {}", std::io::Error::last_os_error());
        return;
    }
    if unsafe { libc::grantpt(fd) } < 0 {
        println!("grantpt error: {}", std::io::Error::last_os_error());
        return;
    }
    if unsafe { libc::unlockpt(fd) } < 0 {
        println!("unlockpt error: {}", std::io::Error::last_os_error());
        return;
    }

    let ws = winsize {
        ws_row: 50,
        ws_col: 110,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    let ret = unsafe { libc::ioctl(fd, TIOCSWINSZ as _, &ws as *const winsize) };
    if ret < 0 {
        println!("ioctl error: {}", std::io::Error::last_os_error());
        return;
    }
    println!("Success");
}
