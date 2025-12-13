use std::{
    ffi::{CString, c_char, c_int},
    fs::OpenOptions,
    io,
    os::fd::{AsRawFd, RawFd},
    path::Path,
};

use anyhow::Result;
use rustix::path::Arg;

const HYMO_IOC_MAGIC: u32 = 0xE0;
const HYMO_IOCTL_HIDE: u64 = ioctl_cmd_write(3, std::mem::size_of::<HymoHide>());
pub(super) const HYMO_DEV: &[&str] = &["/dev/hymo_ctl", "/proc/hymo_ctl"];

#[repr(C)]
struct HymoHide {
    src: *const c_char,
    target: *const c_char,
    r#type: c_int,
}

const fn ioctl_cmd_write(nr: u32, size: usize) -> u64 {
    let size = size as u64;
    (1u32 << 30) as u64 | (size << 16) | ((HYMO_IOC_MAGIC as u64) << 8) | nr as u64
}

fn find_node() -> Option<RawFd> {
    for i in HYMO_DEV {
        if let Ok(dev) = OpenOptions::new().read(true).write(true).open(i) {
            return Some(dev.as_raw_fd());
        }
    }

    None
}

pub(super) fn send_hide_hymofs<P>(target: P) -> Result<()>
where
    P: AsRef<Path>,
{
    let fd = find_node();

    if fd.is_none() {
        return Ok(());
    }
    let fd = fd.unwrap();

    let path = CString::new(target.as_ref().as_str()?)?;
    let cmd = HymoHide {
        src: path.as_ptr(),
        target: std::ptr::null(),
        r#type: 0,
    };

    let ret = unsafe {
        #[cfg(not(target_env = "gnu"))]
        {
            libc::ioctl(fd, HYMO_IOCTL_HIDE as i32, &cmd)
        }
        #[cfg(target_env = "gnu")]
        {
            libc::ioctl(fd, HYMO_IOCTL_HIDE, &cmd)
        }
    };
    if ret < 0 {
        log::error!(
            "umount {} failed: {}",
            target.as_ref().display(),
            io::Error::last_os_error()
        );

        return Ok(());
    }

    log::info!("umount {} successful!", target.as_ref().display());
    Ok(())
}
