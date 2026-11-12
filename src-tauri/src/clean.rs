use crate::{reg::ps, sys};
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

struct Cleaner {
    id: &'static str,
    title: &'static str,
    details: &'static str,
    /// Folders whose contents are deleted. Must start with a %VAR%.
    dirs: &'static [&'static str],
    script: Option<&'static str>,
}

const CLEANERS: &[Cleaner] = &[
    Cleaner {
        id: "temp",
        title: "Temporary files",
        details: "Leftovers from installers and apps in your Temp folder and the Windows Temp folder. Files in use are skipped.",
        dirs: &[r"%TEMP%", r"%SystemRoot%\Temp"],
        script: None,
    },
    Cleaner {
        id: "update-cache",
        title: "Windows Update downloads",
        details: "Installer files for updates that are already installed. Windows downloads them again only if it needs them.",
        dirs: &[r"%SystemRoot%\SoftwareDistribution\Download"],
        script: None,
    },
    Cleaner {
        id: "crash",
        title: "Crash dumps and error reports",
        details: "Memory dumps and reports saved when a program or Windows crashed. Only useful if you are diagnosing a crash.",
        dirs: &[
            r"%LOCALAPPDATA%\CrashDumps",
            r"%ProgramData%\Microsoft\Windows\WER\ReportArchive",
            r"%ProgramData%\Microsoft\Windows\WER\ReportQueue",
            r"%SystemRoot%\Minidump",
            r"%SystemRoot%\LiveKernelReports",
        ],
        script: None,
    },
    Cleaner {
        id: "shader-cache",
        title: "Shader caches",
        details: "Compiled shaders kept by DirectX and the NVIDIA driver. Clear after a driver update or if a game shows graphical glitches or new stutter. Games rebuild them, so expect some stutter the first time you play each game afterwards.",
        dirs: &[r"%LOCALAPPDATA%\D3DSCache", r"%LOCALAPPDATA%\NVIDIA\DXCache", r"%LOCALAPPDATA%\NVIDIA\GLCache"],
        script: None,
    },
    Cleaner {
        id: "delivery-cache",
        title: "Delivery Optimization cache",
        details: "Update pieces Windows kept to share with other PCs.",
        dirs: &[],
        script: Some("Delete-DeliveryOptimizationCache -Force"),
    },
    Cleaner {
        id: "recycle-bin",
        title: "Recycle Bin",
        details: "Permanently deletes everything in the Recycle Bin on all drives.",
        dirs: &[],
        script: Some("Clear-RecycleBin -Force -ErrorAction SilentlyContinue"),
    },
    Cleaner {
        id: "components",
        title: "Old Windows components",
        details: "Removes superseded versions of Windows components left behind by updates. Takes several minutes; the app stays usable while it runs.",
        dirs: &[],
        script: Some("Dism.exe /Online /Cleanup-Image /StartComponentCleanup | Out-Null; if ($LASTEXITCODE -ne 0) { throw ('DISM failed with code ' + $LASTEXITCODE) }"),
    },
];

#[derive(Serialize)]
pub struct Info {
    id: &'static str,
    title: &'static str,
    details: &'static str,
    /// Bytes currently in `dirs`; meaningless when `sized` is false.
    bytes: u64,
    sized: bool,
}

/// `%VAR%\rest` -> absolute path. The base folder comes from Windows, not from the environment
/// variable of that name (see `sys`). None when a folder cannot be trusted, so nothing is ever
/// deleted from a relative or redirected path.
// shortcut: a Temp or AppData folder moved outside the user profile is skipped silently; report it if users move them.
fn expand(p: &str) -> Option<PathBuf> {
    use windows::Win32::UI::Shell::{FOLDERID_LocalAppData, FOLDERID_ProgramData};
    let (var, tail) = p.strip_prefix('%')?.split_once('%')?;
    let base = match var {
        "SystemRoot" => sys::windows_dir()?,
        "ProgramData" => sys::known_folder(&FOLDERID_ProgramData)?,
        "LOCALAPPDATA" => sys::in_profile(sys::known_folder(&FOLDERID_LocalAppData)?)?,
        "TEMP" => sys::in_profile(std::env::temp_dir())?,
        _ => return None,
    };
    Some(base.join(tail.trim_start_matches('\\')))
}

// DirEntry::file_type does not follow links, so links and junctions are never entered or sized.
fn size(dir: &Path) -> u64 {
    let mut total = 0;
    for e in fs::read_dir(dir).into_iter().flatten().flatten() {
        match e.file_type() {
            Ok(t) if t.is_dir() => total += size(&e.path()),
            Ok(t) if t.is_file() => total += e.metadata().map_or(0, |m| m.len()),
            _ => {}
        }
    }
    total
}

/// Deletes what it can inside `dir`; locked files are left behind.
fn empty(dir: &Path) {
    for e in fs::read_dir(dir).into_iter().flatten().flatten() {
        let _ = match e.file_type() {
            Ok(t) if t.is_dir() => fs::remove_dir_all(e.path()),
            Ok(t) if t.is_file() => fs::remove_file(e.path()),
            _ => Ok(()),
        };
    }
}

fn total(c: &Cleaner) -> u64 {
    c.dirs
        .iter()
        .filter_map(|d| expand(d))
        .map(|d| size(&d))
        .sum()
}

pub fn list() -> Vec<Info> {
    CLEANERS
        .iter()
        .map(|c| Info {
            id: c.id,
            title: c.title,
            details: c.details,
            bytes: total(c),
            sized: !c.dirs.is_empty(),
        })
        .collect()
}

/// Returns bytes freed (0 for script-only cleaners, which cannot be measured).
pub fn clean(id: &str) -> Result<u64, String> {
    let c = CLEANERS
        .iter()
        .find(|c| c.id == id)
        .ok_or("unknown cleaner")?;
    let before = total(c);
    c.dirs
        .iter()
        .filter_map(|d| expand(d))
        .for_each(|d| empty(&d));
    if let Some(s) = c.script {
        ps(s)?;
    }
    Ok(before.saturating_sub(total(c)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empties_a_folder_but_keeps_it() {
        let dir = std::env::temp_dir().join("opticlean-selftest");
        fs::create_dir_all(dir.join("sub")).unwrap();
        fs::write(dir.join("a.txt"), "12345").unwrap();
        fs::write(dir.join("sub").join("b.txt"), "123").unwrap();
        assert_eq!(size(&dir), 8);
        empty(&dir);
        assert_eq!(size(&dir), 0);
        assert!(dir.exists());
        fs::remove_dir(&dir).unwrap();
    }

    /// This app runs elevated, and any program running as the user can set environment variables.
    /// A poisoned variable must not be able to point a cleaner at another folder.
    #[test]
    fn system_folders_ignore_environment_variables() {
        let real = expand(r"%SystemRoot%\Temp").unwrap();
        let decoy = std::env::temp_dir().join("opticlean-decoy");
        let saved = std::env::var_os("SystemRoot");
        std::env::set_var("SystemRoot", &decoy);
        let poisoned = expand(r"%SystemRoot%\Temp");
        match saved {
            Some(v) => std::env::set_var("SystemRoot", v),
            None => std::env::remove_var("SystemRoot"),
        }
        assert_eq!(poisoned.unwrap(), real);
        assert!(!real.starts_with(&decoy));
    }

    #[test]
    fn every_folder_resolves_to_an_absolute_path() {
        assert!(expand("relative").is_none());
        assert!(expand(r"%NO_SUCH_VAR_OPTICLEAN%\x").is_none());
        for c in CLEANERS {
            assert!(
                !c.dirs.is_empty() || c.script.is_some(),
                "{} does nothing",
                c.id
            );
            assert!(
                c.script.is_none_or(|s| !s.contains('"')),
                "{} script has a double quote",
                c.id
            );
            for d in c.dirs {
                let p = expand(d).unwrap_or_else(|| panic!("{d} did not expand"));
                assert!(
                    p.is_absolute() && p.components().count() > 2,
                    "{d} is too broad"
                );
            }
        }
    }
}
