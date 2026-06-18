use libc::{
    BRKINT, CS8, ECHO, ICANON, ICRNL, INPCK, ISIG, ISTRIP, IXON, OPOST, STDIN_FILENO, TCSANOW,
    VMIN, VTIME, tcgetattr, tcsetattr, termios,
};
use std::io;

/// RAII guard that puts the terminal into raw mode on creation and restores
/// the original settings on drop.
pub struct RawMode {
    orig: termios,
}

impl RawMode {
    /// Enables raw mode on stdin, returning a guard that restores the previous
    /// settings when dropped.
    pub fn enable() -> io::Result<Self> {
        let mut orig = unsafe { std::mem::zeroed() };
        if unsafe { tcgetattr(STDIN_FILENO, &mut orig) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let mut raw = orig;
        raw.c_cflag &= !(BRKINT | ICRNL | INPCK | ISTRIP | IXON);
        raw.c_oflag &= !(OPOST);
        raw.c_iflag |= CS8;
        raw.c_lflag &= !(ECHO | ICANON | ISIG);
        raw.c_cc[VMIN] = 1;
        raw.c_cc[VTIME] = 0;
        if unsafe { tcsetattr(STDIN_FILENO, TCSANOW, &raw) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(RawMode { orig })
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        unsafe { tcsetattr(STDIN_FILENO, TCSANOW, &self.orig) };
    }
}
