//! Passive physical-pointer observation for safety cancellation.
//!
//! The observer never grabs or consumes a device. It only watches movement
//! events so an accidental real-mouse movement can release Mousetrap's
//! keyboard grab and any held drag. The virtual pointer is excluded by name.

use std::fs::File;
use std::io;
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::fs::OpenOptionsExt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use calloop::channel::Sender;

use crate::input::VIRTUAL_POINTER_NAME;

const IOC_READ: u64 = 2 << 30;
const EVIOCGNAME_BASE: u64 = IOC_READ | (0x45 << 8) | 0x06;
const EVIOCGBIT_BASE: u64 = IOC_READ | (0x45 << 8) | 0x20;

const EV_REL: u16 = 0x02;
const EV_ABS: u16 = 0x03;
const BTN_LEFT: u16 = 0x110;
const REL_X: u16 = 0x00;
const REL_Y: u16 = 0x01;
const ABS_X: u16 = 0x00;
const ABS_Y: u16 = 0x01;
const EVENT_SIZE: usize = 24;

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
    bits.get(bit / 8)
        .map(|byte| byte & (1 << (bit % 8)) != 0)
        .unwrap_or(false)
}

fn device_capabilities(fd: RawFd) -> io::Result<(String, Vec<u8>, Vec<u8>, Vec<u8>)> {
    let mut name = [0u8; 256];
    ioctl(
        fd,
        EVIOCGNAME_BASE | ((name.len() as u64) << 16),
        name.as_mut_ptr().cast(),
    )?;
    let name = String::from_utf8_lossy(&name)
        .trim_end_matches('\0')
        .trim()
        .to_string();

    let mut ev_bits = vec![0u8; 4];
    ioctl(
        fd,
        EVIOCGBIT_BASE | ((ev_bits.len() as u64) << 16),
        ev_bits.as_mut_ptr().cast(),
    )?;
    let mut key_bits = vec![0u8; 96];
    ioctl(
        fd,
        EVIOCGBIT_BASE | ((key_bits.len() as u64) << 16) | 1,
        key_bits.as_mut_ptr().cast(),
    )?;
    let mut rel_bits = vec![0u8; 16];
    ioctl(
        fd,
        EVIOCGBIT_BASE | ((rel_bits.len() as u64) << 16) | 2,
        rel_bits.as_mut_ptr().cast(),
    )?;
    Ok((name, ev_bits, key_bits, rel_bits))
}

fn is_pointer(ev_bits: &[u8], key_bits: &[u8], rel_bits: &[u8]) -> bool {
    let relative = has_bit(ev_bits, EV_REL as usize)
        && has_bit(rel_bits, REL_X as usize)
        && has_bit(rel_bits, REL_Y as usize)
        && has_bit(key_bits, BTN_LEFT as usize);
    let absolute = has_bit(ev_bits, EV_ABS as usize) && has_bit(key_bits, BTN_LEFT as usize);
    relative || absolute
}

fn open_pointers() -> Vec<File> {
    let Ok(entries) = std::fs::read_dir("/dev/input") else {
        return Vec::new();
    };

    entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_name()?.to_str()?;
            if !name.starts_with("event") {
                return None;
            }
            let file = File::options()
                .read(true)
                .custom_flags(libc::O_NONBLOCK)
                .open(path)
                .ok()?;
            let (device_name, ev_bits, key_bits, rel_bits) =
                device_capabilities(file.as_raw_fd()).ok()?;
            if device_name == VIRTUAL_POINTER_NAME || !is_pointer(&ev_bits, &key_bits, &rel_bits) {
                return None;
            }
            Some(file)
        })
        .collect()
}

fn reader_loop(files: Vec<File>, stop: Arc<AtomicBool>, tx: Sender<MouseEvent>) {
    let mut pollfds: Vec<libc::pollfd> = files
        .iter()
        .map(|file| libc::pollfd {
            fd: file.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        })
        .collect();
    let mut buffer = [0u8; EVENT_SIZE];

    while !stop.load(Ordering::Relaxed) {
        let ready = unsafe { libc::poll(pollfds.as_mut_ptr(), pollfds.len() as libc::nfds_t, 100) };
        if ready <= 0 {
            continue;
        }
        for pollfd in &mut pollfds {
            if pollfd.revents & libc::POLLIN == 0 {
                continue;
            }
            for _ in 0..32 {
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                let read = unsafe {
                    libc::read(
                        pollfd.fd,
                        buffer.as_mut_ptr().cast::<libc::c_void>(),
                        EVENT_SIZE,
                    )
                };
                if read != EVENT_SIZE as isize {
                    break;
                }
                let event =
                    unsafe { std::ptr::read_unaligned(buffer.as_ptr().cast::<InputEvent>()) };
                let movement = (event.type_ == EV_REL
                    && (event.code == REL_X || event.code == REL_Y)
                    && event.value != 0)
                    || (event.type_ == EV_ABS && (event.code == ABS_X || event.code == ABS_Y));
                if movement {
                    let _ = tx.send(MouseEvent::Moved);
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MouseEvent {
    Moved,
}

/// A detached, non-invasive observer. Dropping it asks the reader to stop;
/// the short poll interval keeps teardown prompt without joining a thread.
pub struct MouseObserver {
    stop: Arc<AtomicBool>,
}

impl MouseObserver {
    pub fn start(tx: Sender<MouseEvent>) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = stop.clone();
        std::thread::spawn(move || reader_loop(open_pointers(), thread_stop, tx));
        Self { stop }
    }
}

impl Drop for MouseObserver {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}
