mod hymofs;
mod kernel;

use std::{
    fs::{self, read_dir},
    path::Path,
};

use anyhow::Result;

use crate::defs::{DISABLE_FILE_NAME, REMOVE_FILE_NAME, SKIP_MOUNT_FILE_NAME};

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

    for i in hymofs::HYMO_DEV {
        if fs::exists(i)? {
            hymofs::send_hide_hymofs(target.as_ref())?;
            return Ok(());
        }
    }

    kernel::send_kernel_umount(target.as_ref())?;
    Ok(())
}
