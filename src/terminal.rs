use libc::{
    POLLIN, SA_RESTART, SIGWINCH, STDIN_FILENO, STDOUT_FILENO, TIOCGWINSZ, c_int, c_void, ioctl,
    pipe, poll, pollfd, read, sigaction, sigemptyset, sighandler_t, winsize, write,
};
use std::io::{self, Write};
use std::sync::atomic::{AtomicI32, Ordering};

use crate::raw_mode::RawMode;

/// Dimensions of the terminal, in character cells.
#[derive(Clone, Copy)]
pub struct TermSize {
    pub rows: u16,
    pub cols: u16,
}

/// A key press decoded from the input stream.
///
/// TODO: extend with the remaining keys (function keys, Home/End, modifiers,
/// ...) once the input parser is written.
pub enum Key {
    Char(char),
    Up,
    Down,
    Left,
    Right,
    Enter,
    Esc,
}

/// An event produced by [`Terminal::next_event`].
pub enum Event {
    Key(Key),
    Resize(TermSize),
}

/// Owns the TTY session: raw mode, the alternate screen, and the resize
/// self-pipe. Constructing one sets the terminal up; dropping it restores the
/// terminal to its original state.
pub struct Terminal {
    _raw: RawMode,
    pipe_read_fd: i32,
    size: TermSize,
}

impl Terminal {
    /// Enters raw mode and the alternate screen, installs the resize handler,
    /// and reads the initial size.
    pub fn new() -> io::Result<Self> {
        let _raw = RawMode::enable()?;
        // Enter the alternate screen and clear it.
        print!("\x1b[?1049h\x1b[2J\x1b[H");
        io::stdout().flush()?;
        let pipe_read_fd = setup_resize_pipe()?;
        let size = get_window_size()?;
        Ok(Terminal {
            _raw,
            pipe_read_fd,
            size,
        })
    }

    /// The current terminal size, kept fresh by [`Terminal::refresh_size`].
    pub fn size(&self) -> &TermSize {
        &self.size
    }

    /// Read end of the resize self-pipe; poll it for `SIGWINCH` notifications.
    pub fn resize_fd(&self) -> i32 {
        self.pipe_read_fd
    }

    /// Re-reads the terminal size. Call after the resize pipe signals.
    pub fn refresh_size(&mut self) -> io::Result<()> {
        self.size = get_window_size()?;
        Ok(())
    }

    /// Waits for the next input or resize event.
    ///
    /// `timeout_ms` is the maximum time to wait: `None` blocks indefinitely,
    /// `Some(0)` polls without blocking. Returns `Ok(None)` if the timeout
    /// elapses with no event. `EINTR` is retried internally and never surfaces.
    pub fn next_event(&mut self, timeout_ms: Option<u32>) -> io::Result<Option<Event>> {
        let timeout = timeout_ms.map_or(-1, |ms| ms as c_int);
        loop {
            let mut fds = [
                pollfd {
                    fd: self.resize_fd(),
                    events: POLLIN,
                    revents: 0,
                },
                pollfd {
                    fd: STDIN_FILENO,
                    events: POLLIN,
                    revents: 0,
                },
            ];

            let ret = unsafe { poll(fds.as_mut_ptr(), fds.len() as _, timeout) };
            if ret == -1 {
                let err = io::Error::last_os_error();
                if err.kind() == io::ErrorKind::Interrupted {
                    continue; // retry; a signal interrupted the wait
                }
                return Err(err);
            }
            if ret == 0 {
                return Ok(None); // timed out with no event
            }

            // SIGWINCH woke the poll: drain the byte and re-measure.
            if fds[0].revents & POLLIN != 0 {
                let mut buf = [0u8; 64];
                if unsafe { read(fds[0].fd, buf.as_mut_ptr() as *mut c_void, buf.len()) } == -1 {
                    let err = io::Error::last_os_error();
                    if err.kind() == io::ErrorKind::Interrupted {
                        continue;
                    }
                    return Err(err);
                }
                self.refresh_size()?;
                return Ok(Some(Event::Resize(*self.size())));
            }

            // Keyboard input is ready.
            if fds[1].revents & POLLIN != 0 {
                let mut buf = [0u8; 1];
                let n = unsafe { read(STDIN_FILENO, buf.as_mut_ptr() as *mut c_void, 1) };
                if n == -1 {
                    let err = io::Error::last_os_error();
                    if err.kind() == io::ErrorKind::Interrupted {
                        continue;
                    }
                    return Err(err);
                }
                if n == 0 {
                    return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
                }
                // TODO: decode escape sequences into the full `Key` set; for now
                // pass the raw byte through as a character.
                return Ok(Some(Event::Key(Key::Char(buf[0] as char))));
            }
        }
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        // Leave the alternate screen; `RawMode`'s Drop then restores cooked mode.
        print!("\x1b[?1049l");
        let _ = io::stdout().flush();
    }
}

// The items below live at module scope, outside `impl Terminal`, because they
// exist to serve an OS signal handler — and a signal handler cannot be a method:
//
//   * `handle_sigwinch` is an `extern "C"` function the kernel invokes directly
//     via a C function pointer. It has no `self` and no captured environment, so
//     it cannot reach a `Terminal` instance. It must be a free function.
//   * Because the handler has no `self`, the write end of the pipe it needs is
//     reached through this file-scope `static` instead. (It is an `AtomicI32`
//     so the store/load is async-signal-safe.)
//   * `setup_resize_pipe` installs that handler and seeds the `static`, so it is
//     grouped here with the handler and the global it owns rather than in the impl.
//
// `Terminal::new` calls into these; they are the private signal-handling
// machinery behind the public API above.

/// Write end of the self-pipe, shared with the signal handler so it can wake
/// the poll loop on `SIGWINCH`.
static PIPE_WRITE_FD: AtomicI32 = AtomicI32::new(-1);

/// Installs a `SIGWINCH` handler that notifies via a self-pipe.
///
/// Returns the read end of the pipe, which the caller polls for resize events.
fn setup_resize_pipe() -> io::Result<i32> {
    let mut fds = [-1i32; 2];
    if unsafe { pipe(fds.as_mut_ptr()) } == -1 {
        return Err(io::Error::last_os_error());
    }

    let (read_fd, write_fd) = (fds[0], fds[1]);
    PIPE_WRITE_FD.store(write_fd, Ordering::Relaxed);

    let mut sa: sigaction = unsafe { std::mem::zeroed() };
    sa.sa_sigaction = handle_sigwinch as *const () as sighandler_t;
    unsafe { sigemptyset(&mut sa.sa_mask) };
    sa.sa_flags = SA_RESTART;

    if unsafe { sigaction(SIGWINCH, &sa, std::ptr::null_mut()) } == -1 {
        return Err(io::Error::last_os_error());
    }

    Ok(read_fd)
}

/// Handles `SIGWINCH` by writing a single byte to the self-pipe.
///
/// Only async-signal-safe operations are used here: `libc::write` is the sole
/// call, and it is strictly async-signal-safe.
extern "C" fn handle_sigwinch(_sig: c_int) {
    let fd = PIPE_WRITE_FD.load(Ordering::Relaxed);
    if fd > 0 {
        let buf: [u8; 1] = [1];
        unsafe {
            write(fd, buf.as_ptr() as *const libc::c_void, 1);
        }
    }
}

/// Queries the current terminal size via the `TIOCGWINSZ` ioctl.
fn get_window_size() -> io::Result<TermSize> {
    let mut ws: winsize = unsafe { std::mem::zeroed() };
    let ret = unsafe { ioctl(STDOUT_FILENO, TIOCGWINSZ, &mut ws) };
    if ret == -1 {
        Err(io::Error::other("ioctl TIOCGWINSZ failed"))
    } else {
        Ok(TermSize {
            rows: ws.ws_row,
            cols: ws.ws_col,
        })
    }
}
