use std::{
    ffi::CString,
    fs::{self, read_dir},
    io,
    os::fd::RawFd,
    path::Path,
    sync::OnceLock,
};

use anyhow::Result;
use rustix::path::Arg;

use crate::defs::{DISABLE_FILE_NAME, REMOVE_FILE_NAME, SKIP_MOUNT_FILE_NAME};

const K: u32 = b'K' as u32;
const KSU_INSTALL_MAGIC1: u32 = 0xDEAD_BEEF;
#[cfg(target_env = "gnu")]
const KSU_IOCTL_ADD_TRY_UMOUNT: u64 = libc::_IOW::<()>(K, 18);
#[cfg(not(target_env = "gnu"))]
const KSU_IOCTL_ADD_TRY_UMOUNT: i32 = libc::_IOW::<()>(K, 18);
const KSU_INSTALL_MAGIC2: u32 = 0xCAFE_BABE;
static DRIVER_FD: OnceLock<RawFd> = OnceLock::new();

#[repr(C)]
struct KsuAddTryUmount {
    arg: u64,
    flags: u32,
    mode: u8,
}

pub fn send_unmountable<P>(target: P) -> Result<()>
where
    P: AsRef<Path>,
{
    for entry in read_dir("/data/adb/modules")?.flatten() {
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        if !path.join("module.prop").exists() {
            continue;
        }

        let disabled =
            path.join(DISABLE_FILE_NAME).exists() || path.join(REMOVE_FILE_NAME).exists();
        let skip = path.join(SKIP_MOUNT_FILE_NAME).exists();
        if disabled || skip {
            continue;
        }

        if let Some(name) = path.file_name()
            && name.to_string_lossy().to_string().contains("zygisksu")
            && fs::read_to_string("/data/adb/zygisksu/denylist_enforce")?.trim() == "0"
        {
            log::warn!("zn was detected, and try_umount was cancelled.");
            return Ok(());
        }
    }

    let path = CString::new(target.as_ref().as_str()?)?;
    let cmd = KsuAddTryUmount {
        arg: path.as_ptr() as u64,
        flags: 2,
        mode: 1,
    };

    let fd = *DRIVER_FD.get_or_init(|| {
        let mut fd = -1;
        unsafe {
            libc::syscall(
                libc::SYS_reboot,
                KSU_INSTALL_MAGIC1,
                KSU_INSTALL_MAGIC2,
                0,
                &mut fd,
            );
        };
        fd
    });

    unsafe {
        #[cfg(target_env = "gnu")]
        let ret = libc::ioctl(fd as libc::c_int, KSU_IOCTL_ADD_TRY_UMOUNT, &cmd);

        #[cfg(not(target_env = "gnu"))]
        let ret = libc::ioctl(fd as libc::c_int, KSU_IOCTL_ADD_TRY_UMOUNT, &cmd);

        if ret < 0 {
            log::error!(
                "umount {} failed: {}",
                target.as_ref().display(),
                io::Error::last_os_error()
            );

            return Ok(());
        }

        log::info!("umount {} successful!", target.as_ref().display());
    };

    Ok(())
}
