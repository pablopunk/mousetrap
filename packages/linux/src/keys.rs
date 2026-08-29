//! Keyboard capture via evdev + `EVIOCGRAB`.
//!
//! While the grid is active, the physical keyboard(s) are exclusively
//! grabbed at the kernel level: keystrokes reach us and no other reader —
//! including the compositor and the focused app (the Wayland equivalent of
//! the macOS CGEvent tap). The grab is released on commit, cancel, timeout,
//! quit, or process death (the kernel clears grabs when the fd closes).
//!
//! Requires read/write access to `/dev/input/event*` (the `input` group) —
//! the same permission class as `/dev/uinput` for clicks.

use std::fs::File;
use std::io;
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::fs::OpenOptionsExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use calloop::channel::Sender;

// linux/input.h ioctl requests (64-bit).
const IOC_READ: u64 = 2 << 30;
const EVIOCGNAME_BASE: u64 = IOC_READ | (0x45 << 8) | 0x06;
const EVIOCGRAB: u64 = (1 << 30) | (0x45 << 8) | 0x90 | (4 << 16);
const EVIOCGBIT_BASE: u64 = IOC_READ | (0x45 << 8) | 0x20;

// linux/input-event-codes.h
const EV_KEY: u16 = 0x01;
const KEY_ESC: u16 = 1;
const KEY_1: u16 = 2;
const KEY_0: u16 = 11;
const KEY_Q: u16 = 16;
const KEY_P: u16 = 25;
const KEY_A: u16 = 30;
const KEY_L: u16 = 38;
const KEY_SEMICOLON: u16 = 39;
const KEY_Z: u16 = 44;
const KEY_M: u16 = 50;
const KEY_COMMA: u16 = 51;
const KEY_DOT: u16 = 52;
const KEY_SLASH: u16 = 53;
const KEY_ENTER: u16 = 28;
const KEY_SPACE: u16 = 57;
const BTN_LEFT: u16 = 0x110;

const EVENT_SIZE: usize = 24; // struct input_event on 64-bit

/// Events forwarded from the keyboard grab thread to the main loop.
#[derive(Debug, Clone)]
pub enum KeyEvent {
    KeyDown(String),
    KeyUp(String),
    /// Escape pressed while the grid is active (cancels the session).
    Escape,
}

/// Map an evdev keycode to a grid key. `None` = not a grid key (still
/// swallowed by the grab). Rows are looked up positionally because the
/// keyboard rows are not alphabetical (qwertyuiop, asdfghjkl;, zxcvbnm,./).
fn keycode_to_grid(code: u16) -> Option<char> {
    const ROW0: &str = "1234567890";
    const ROW1: &str = "qwertyuiop";
    const ROW2: &str = "asdfghjkl;";
    const ROW3: &str = "zxcvbnm,./";
    match code {
        KEY_1..=KEY_0 => ROW0.chars().nth((code - KEY_1) as usize),
        KEY_Q..=KEY_P => ROW1.chars().nth((code - KEY_Q) as usize),
        KEY_A..=KEY_L => ROW2.chars().nth((code - KEY_A) as usize),
        KEY_SEMICOLON => Some(';'),
        KEY_Z..=KEY_M => ROW3.chars().nth((code - KEY_Z) as usize),
        KEY_COMMA => Some(','),
        KEY_DOT => Some('.'),
        KEY_SLASH => Some('/'),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keycode_mapping_follows_grid_rows() {
        assert_eq!(keycode_to_grid(KEY_Q), Some('q'));
        assert_eq!(keycode_to_grid(23), Some('i')); // KEY_I
        assert_eq!(keycode_to_grid(33), Some('f')); // KEY_F
        assert_eq!(keycode_to_grid(KEY_SEMICOLON), Some(';'));
        assert_eq!(keycode_to_grid(48), Some('b')); // KEY_B
        assert_eq!(keycode_to_grid(KEY_COMMA), Some(','));
        assert_eq!(keycode_to_grid(KEY_DOT), Some('.'));
        assert_eq!(keycode_to_grid(KEY_SLASH), Some('/'));
        assert_eq!(keycode_to_grid(KEY_1), Some('1'));
        assert_eq!(keycode_to_grid(KEY_0), Some('0'));
        assert_eq!(keycode_to_grid(KEY_ESC), None);
        assert_eq!(keycode_to_grid(KEY_ENTER), None);
    }
}

#[repr(C)]
struct InputEvent {
    time: libc::timeval,
    type_: u16,
    code: u16,
    value: i32,
}

fn ioctl(fd: RawFd, request: u64, arg: *mut libc::c_void) -> io::Result<()> {
    let ret = unsafe { libc::ioctl(fd, request as libc::c_ulong, arg) };
    if ret < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn has_bit(bits: &[u8], bit: usize) -> bool {
    bits.get(bit / 8).map(|b| b & (1 << (bit % 8)) != 0).unwrap_or(false)
}

fn device_capabilities(fd: RawFd) -> io::Result<(String, Vec<u8>, Vec<u8>)> {
    let mut name = [0u8; 256];
    ioctl(fd, EVIOCGNAME_BASE | ((name.len() as u64) << 16), name.as_mut_ptr().cast())?;
    let name = String::from_utf8_lossy(&name)
        .trim_end_matches('\0')
        .trim()
        .to_string();
    let mut ev_bits = vec![0u8; 4]; // EV_* up to 31
    ioctl(
        fd,
        EVIOCGBIT_BASE | ((ev_bits.len() as u64) << 16),
        ev_bits.as_mut_ptr().cast(),
    )?;
    let mut key_bits = vec![0u8; 96]; // KEY_* up to 767
    ioctl(
        fd,
        EVIOCGBIT_BASE | ((key_bits.len() as u64) << 16) | 1,
        key_bits.as_mut_ptr().cast(),
    )?;
    Ok((name, ev_bits, key_bits))
}

fn is_keyboard(ev_bits: &[u8], key_bits: &[u8]) -> bool {
    // Must report key events and have a keyboard-like key set.
    if !has_bit(ev_bits, EV_KEY as usize) {
        return false;
    }
    let letters = (KEY_A..=KEY_Z).all(|k| has_bit(key_bits, k as usize));
    let digits = (KEY_1..=KEY_0).all(|k| has_bit(key_bits, k as usize));
    let structural =
        has_bit(key_bits, KEY_ENTER as usize) && has_bit(key_bits, KEY_SPACE as usize);
    // Mice and trackballs also report letters/digits through their receiver;
    // exclude anything with mouse buttons so we never swallow clicks.
    let mouse_like = has_bit(key_bits, BTN_LEFT as usize);
    letters && digits && structural && !mouse_like
}

/// Open and grab every keyboard-like device.
fn open_keyboards() -> io::Result<Vec<File>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir("/dev/input")? {
        let path = entry?.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.starts_with("event") {
            continue;
        }
        let file = match File::options()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(&path)
        {
            Ok(f) => f,
            Err(_) => continue, // no access (e.g. touchpads owned elsewhere)
        };
        let fd = file.as_raw_fd();
        let Ok((_name, ev_bits, key_bits)) = device_capabilities(fd) else {
            continue;
        };
        if !is_keyboard(&ev_bits, &key_bits) {
            continue;
        }
        match ioctl(fd, EVIOCGRAB, 1usize as *mut libc::c_void) {
            Ok(()) => files.push(file),
            Err(e) => {
                eprintln!("mousetrap: cannot grab {}: {e}", path.display());
            }
        }
    }
    Ok(files)
}

fn reader_loop(
    files: Vec<File>,
    stop: Arc<AtomicBool>,
    tx: Sender<KeyEvent>,
    timeout_seconds: f64,
    processed: Arc<std::sync::atomic::AtomicU64>,
) {
    let mut pollfds: Vec<libc::pollfd> = files
        .iter()
        .map(|f| libc::pollfd {
            fd: f.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        })
        .collect();
    let mut buf = [0u8; EVENT_SIZE];
    let grace = Duration::from_secs(2);
    // Failsafe independent of the main loop: if the main loop stops
    // processing forwarded keys (wedge), release the grabs (via Drop) and
    // kill the daemon. The deadline tracks the main loop's own heartbeat,
    // so healthy sessions never trip it.
    let deadline = || {
        let last = processed.load(Ordering::Relaxed);
        let last = if last == 0 {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0)
        } else {
            last
        };
        last + Duration::from_secs_f64(timeout_seconds.max(1.0)).as_nanos() as u64 + grace.as_nanos() as u64
    };
    while !stop.load(Ordering::Relaxed) {
        let now_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        if now_ns >= deadline() {
            eprintln!("mousetrap: keyboard grab deadline exceeded; releasing and exiting");
            std::process::exit(1);
        }
        let ready = unsafe { libc::poll(pollfds.as_mut_ptr(), pollfds.len() as libc::nfds_t, 100) };
        if ready <= 0 {
            continue;
        }
        for pfd in &mut pollfds {
            if pfd.revents & libc::POLLIN == 0 {
                continue;
            }
            // Drain the queue, but re-check the stop flag and yield to the
            // outer checks frequently: a continuously-typing user must never
            // starve the stop flag (this locked the keyboard in the past).
            for _ in 0..32 {
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                let read = unsafe {
                    libc::read(pfd.fd, buf.as_mut_ptr().cast::<libc::c_void>(), EVENT_SIZE)
                };
                if read != EVENT_SIZE as isize {
                    break; // EAGAIN or drained
                }
                let event = unsafe { std::ptr::read_unaligned(buf.as_ptr().cast::<InputEvent>()) };
                if event.type_ != EV_KEY {
                    continue;
                }
                match event.value {
                    1 => {
                        if event.code == KEY_ESC {
                            let _ = tx.send(KeyEvent::Escape);
                        } else if let Some(key) = keycode_to_grid(event.code) {
                            let _ = tx.send(KeyEvent::KeyDown(key.to_string()));
                        }
                    }
                    0 => {
                        if let Some(key) = keycode_to_grid(event.code) {
                            let _ = tx.send(KeyEvent::KeyUp(key.to_string()));
                        }
                    }
                    _ => {} // repeats (2) and others are ignored
                }
            }
        }
    }
    // Dropping `files` releases every grab.
}

/// Active keyboard grab; released on `stop`/drop.
pub struct KeyboardGrab {
    stop: Arc<AtomicBool>,
}

impl KeyboardGrab {
    /// Grab all keyboards and start forwarding events. Returns `Err` when no
    /// keyboard could be grabbed (e.g. missing `input` group membership).
    ///
    /// `processed` is a heartbeat the main loop updates when it handles key
    /// events; the reader's failsafe deadline is anchored to it, so the grab
    /// can never outlive a wedged main loop.
    pub fn start(
        tx: Sender<KeyEvent>,
        session_timeout_seconds: f64,
        processed: Arc<std::sync::atomic::AtomicU64>,
    ) -> Result<Self, String> {
        let files = open_keyboards().map_err(|e| format!("cannot access /dev/input: {e}"))?;
        if files.is_empty() {
            return Err(
                "no keyboards found — add your user to the `input` group (this is Mousetrap's \
                 equivalent of the macOS Accessibility permission)"
                    .to_string(),
            );
        }
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = stop.clone();
        // Detached: the reader enforces its own deadline and releases the
        // grabs on its own. The main loop must never wait on this thread.
        std::thread::spawn(move || {
            reader_loop(files, thread_stop, tx, session_timeout_seconds, processed)
        });
        Ok(Self { stop })
    }

    /// Signal the reader thread to release all grabs. Never blocks.
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

impl Drop for KeyboardGrab {
    fn drop(&mut self) {
        self.stop();
    }
}
