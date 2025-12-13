use std::{ffi::CString, io, os::fd::RawFd, path::Path, sync::OnceLock};

use anyhow::Result;
use rustix::path::Arg;

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

pub(super) fn send_kernel_umount<P>(target: P) -> Result<()>
where
    P: AsRef<Path>,
{
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
