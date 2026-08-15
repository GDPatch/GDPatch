use std::{
    path::{Path, PathBuf},
    time::SystemTime,
};

use libc::{c_int, time_t};

use crate::Filesilly;

pub fn mark_errno(value: c_int) {
    unsafe {
        *libc::__errno_location() = value;
    }
}

pub fn resolve_path(path: &Path) -> Option<PathBuf> {
    let path = jank_canonicalize(path);

    let platform = Filesilly::platform();
    platform.base_paths.iter().find_map(|base_path| {
        let relative_path = path.strip_prefix(base_path).ok()?;
        let absolute_path = base_path.join(relative_path);
        Some(absolute_path)
    })
}

pub fn jank_canonicalize(path: &Path) -> PathBuf {
    if !path.is_absolute()
        && let Ok(current_dir) = std::env::current_dir()
    {
        current_dir.join(path)
    } else {
        path.to_owned()
    }
}

pub fn system_time_to_timespec(time: SystemTime) -> (time_t, i64) {
    let since_epoch = time
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();

    (
        since_epoch.as_secs() as time_t,
        since_epoch.subsec_nanos() as i64,
    )
}
