//! Folders as Windows itself reports them, never from environment variables.
//! OptiClean runs elevated, and any program running as the user can change that user's
//! environment variables. A path built from one could send an elevated delete, or an elevated
//! program launch, somewhere else.

use std::path::PathBuf;
use windows::{
    core::GUID,
    Win32::{
        System::{
            Com::CoTaskMemFree,
            SystemInformation::{GetSystemDirectoryW, GetSystemWindowsDirectoryW},
        },
        UI::Shell::{FOLDERID_Profile, SHGetKnownFolderPath, KF_FLAG_DEFAULT},
    },
};

fn from_api(fill: unsafe fn(Option<&mut [u16]>) -> u32) -> Option<PathBuf> {
    let mut buf = [0u16; 260];
    let len = unsafe { fill(Some(&mut buf)) } as usize;
    // 0 is failure and a longer length means the buffer was too small: never return a partial path.
    (1..buf.len())
        .contains(&len)
        .then(|| PathBuf::from(String::from_utf16_lossy(&buf[..len])))
}

/// `C:\Windows`
pub fn windows_dir() -> Option<PathBuf> {
    from_api(GetSystemWindowsDirectoryW)
}

/// `C:\Windows\System32`
pub fn system_dir() -> Option<PathBuf> {
    from_api(GetSystemDirectoryW)
}

pub fn known_folder(id: &GUID) -> Option<PathBuf> {
    unsafe {
        let raw = SHGetKnownFolderPath(id, KF_FLAG_DEFAULT, None).ok()?;
        let path = raw.to_string().ok().map(PathBuf::from);
        CoTaskMemFree(Some(raw.0 as _));
        path
    }
}

/// `dir` with links resolved, but only when it really is inside the signed-in user's profile.
/// Per-user folders can be redirected by the user, or by anything running as them.
pub fn in_profile(dir: PathBuf) -> Option<PathBuf> {
    let profile = known_folder(&FOLDERID_Profile)?.canonicalize().ok()?;
    let dir = dir.canonicalize().ok()?;
    dir.starts_with(&profile).then_some(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_folders_come_from_windows() {
        let windows = windows_dir().unwrap();
        let system = system_dir().unwrap();
        assert!(windows.is_absolute() && system.starts_with(&windows));
        assert!(system.join(r"WindowsPowerShell\v1.0\powershell.exe").exists());
    }

    #[test]
    fn only_folders_inside_the_profile_are_accepted() {
        assert!(in_profile(std::env::temp_dir()).is_some());
        assert!(in_profile(windows_dir().unwrap()).is_none());
        assert!(in_profile(PathBuf::from(r"C:\no\such\folder")).is_none());
    }
}
