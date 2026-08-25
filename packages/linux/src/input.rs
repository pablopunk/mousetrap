//! Virtual pointer device via `/dev/uinput`.
//!
//! Wayland has no client-side API for cursor warping or click injection, so
//! we create a kernel-level virtual input device. Every Wayland compositor
//! consumes it like a real mouse (this is the same mechanism ydotool uses,
//! but embedded here so the binary has no external dependencies).
//!
//! Requires write access to `/dev/uinput` — see the udev rule in the README.
//! This is the Wayland equivalent of the macOS Accessibility permission.

use std::fs::File;
use std::io::{self, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;

// linux/uinput.h
const UI_SET_EVBIT: u64 = 0x4004_5564; // _IOW('U', 100, int)
const UI_SET_KEYBIT: u64 = 0x4004_5565; // _IOW('U', 101, int)
const UI_SET_ABSBIT: u64 = 0x4004_5567; // _IOW('U', 103, int)
const UI_DEV_CREATE: u64 = 0x5501; // _IO('U', 1)
const UI_DEV_DESTROY: u64 = 0x5502; // _IO('U', 2)

// linux/input-event-codes.h
const EV_SYN: u16 = 0x00;
const EV_KEY: u16 = 0x01;
const EV_ABS: u16 = 0x03;
const SYN_REPORT: u16 = 0x00;
const BTN_LEFT: u16 = 0x110;
const BTN_RIGHT: u16 = 0x111;
const BTN_MIDDLE: u16 = 0x112;
const ABS_X: usize = 0x00;
const ABS_Y: usize = 0x01;
const ABS_CNT: usize = 0x40;

const UINPUT_MAX_NAME_SIZE: usize = 80;
const BUS_USB: u16 = 0x03;
const ABS_MAX: i32 = 32767;

#[repr(C)]
struct InputId {
    bustype: u16,
    vendor: u16,
    product: u16,
    version: u16,
}

#[repr(C)]
struct UInputUserDev {
    name: [u8; UINPUT_MAX_NAME_SIZE],
    id: InputId,
    ff_effects_max: u32,
    absmax: [i32; ABS_CNT],
    absmin: [i32; ABS_CNT],
    absfuzz: [i32; ABS_CNT],
    absflat: [i32; ABS_CNT],
}

/// One `struct input_event` (24 bytes on 64-bit Linux).
#[repr(C)]
struct InputEvent {
    time: libc::timeval,
    type_: u16,
    code: u16,
    value: i32,
}

unsafe fn ioctl_u32(fd: std::os::fd::RawFd, request: u64, value: u32) -> io::Result<()> {
    let ret = unsafe { libc::ioctl(fd, request as libc::c_ulong, value) };
    if ret < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

pub struct VirtualPointer {
    file: File,
}

impl VirtualPointer {
    pub fn new() -> io::Result<Self> {
        let file = File::options()
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open("/dev/uinput")
            .map_err(|e| {
                if e.kind() == io::ErrorKind::PermissionDenied {
                    io::Error::new(
                        e.kind(),
                        "no write access to /dev/uinput — add a udev rule or add your user \
                         to the `input` group (see README). This is Mousetrap's equivalent \
                         of the macOS Accessibility permission.",
                    )
                } else {
                    e
                }
            })?;
        let fd = file.as_raw_fd();

        let mut dev = UInputUserDev {
            name: [0; UINPUT_MAX_NAME_SIZE],
            id: InputId {
                bustype: BUS_USB,
                vendor: 0x1234,
                product: 0x0001,
                version: 1,
            },
            ff_effects_max: 0,
            absmax: [0; ABS_CNT],
            absmin: [0; ABS_CNT],
            absfuzz: [0; ABS_CNT],
            absflat: [0; ABS_CNT],
        };
        let name = b"mousetrap virtual pointer";
        dev.name[..name.len()].copy_from_slice(name);
        dev.absmax[ABS_X] = ABS_MAX;
        dev.absmax[ABS_Y] = ABS_MAX;

        unsafe {
            ioctl_u32(fd, UI_SET_EVBIT, EV_KEY as u32)?;
            ioctl_u32(fd, UI_SET_EVBIT, EV_ABS as u32)?;
            ioctl_u32(fd, UI_SET_EVBIT, EV_SYN as u32)?;
            ioctl_u32(fd, UI_SET_KEYBIT, BTN_LEFT as u32)?;
            ioctl_u32(fd, UI_SET_KEYBIT, BTN_RIGHT as u32)?;
            ioctl_u32(fd, UI_SET_KEYBIT, BTN_MIDDLE as u32)?;
            ioctl_u32(fd, UI_SET_ABSBIT, ABS_X as u32)?;
            ioctl_u32(fd, UI_SET_ABSBIT, ABS_Y as u32)?;
        }

        let mut pointer = Self { file };
        // The device description must be written BEFORE UI_DEV_CREATE.
        pointer.write_dev(&dev)?;
        unsafe {
            ioctl_u32(pointer.file.as_raw_fd(), UI_DEV_CREATE, 0)?;
        }
        Ok(pointer)
    }

    fn write_dev(&mut self, dev: &UInputUserDev) -> io::Result<()> {
        let bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(
                (dev as *const UInputUserDev).cast::<u8>(),
                std::mem::size_of::<UInputUserDev>(),
            )
        };
        self.file.write_all(bytes)
    }

    fn write_event(&mut self, type_: u16, code: u16, value: i32) -> io::Result<()> {
        let ev = InputEvent {
            time: libc::timeval { tv_sec: 0, tv_usec: 0 },
            type_,
            code,
            value,
        };
        let bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(
                (&ev as *const InputEvent).cast::<u8>(),
                std::mem::size_of::<InputEvent>(),
            )
        };
        self.file.write_all(bytes)
    }

    fn emit(&mut self, type_: u16, code: u16, value: i32) -> io::Result<()> {
        self.write_event(type_, code, value)?;
        self.write_event(EV_SYN, SYN_REPORT, 0)
    }

    /// Move the pointer to absolute logical coordinates within a screen of
    /// `screen_w` x `screen_h`. libinput normalizes the ABS range onto the
    /// output the pointer is on.
    pub fn move_abs(&mut self, x: i32, y: i32, screen_w: i32, screen_h: i32) -> io::Result<()> {
        let scale = |v: i32, max: i32| -> i32 {
            ((v as f64 / max.max(1) as f64 * ABS_MAX as f64).clamp(0.0, ABS_MAX as f64)) as i32
        };
        self.write_event(EV_ABS, ABS_X as u16, scale(x, screen_w))?;
        self.write_event(EV_ABS, ABS_Y as u16, scale(y, screen_h))?;
        self.write_event(EV_SYN, SYN_REPORT, 0)
    }

    pub fn click(&mut self, button: u16) -> io::Result<()> {
        self.emit(EV_KEY, button, 1)?;
        self.emit(EV_KEY, button, 0)
    }

    pub fn double_click(&mut self, button: u16, interval_seconds: f64) -> io::Result<()> {
        self.click(button)?;
        std::thread::sleep(std::time::Duration::from_secs_f64(interval_seconds));
        self.click(button)
    }

    pub fn left_click(&mut self) -> io::Result<()> {
        self.click(BTN_LEFT)
    }

    pub fn right_click(&mut self) -> io::Result<()> {
        self.click(BTN_RIGHT)
    }

    pub fn drag_start(&mut self, button: u16) -> io::Result<()> {
        self.emit(EV_KEY, button, 1)
    }

    pub fn drag_end(&mut self, button: u16) -> io::Result<()> {
        self.emit(EV_KEY, button, 0)
    }
}

impl Drop for VirtualPointer {
    fn drop(&mut self) {
        unsafe {
            let _ = libc::ioctl(self.file.as_raw_fd(), UI_DEV_DESTROY as libc::c_ulong, 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uinput_device_creates() {
        let pointer = VirtualPointer::new();
        assert!(pointer.is_ok(), "VirtualPointer::new failed: {:?}", pointer.err());
    }
}
