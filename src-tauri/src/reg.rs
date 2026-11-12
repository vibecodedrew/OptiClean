use std::{io, os::windows::process::CommandExt, process::Command};
use winreg::{enums::*, RegKey, RegValue};

#[derive(Clone, Copy)]
pub enum Hive {
    Hkcu,
    Hklm,
}

pub enum Val {
    Dword(u32),
    Str(&'static str),
    Bin(&'static [u8]),
    Delete,
}

pub enum Action {
    Reg {
        hive: Hive,
        path: &'static str,
        name: &'static str,
        on: Val,
        off: Val,
    },
    /// PowerShell. `check` prints `True` when applied. Scripts must not contain double quotes (see `ps`).
    Script {
        apply: &'static str,
        undo: &'static str,
        check: &'static str,
    },
}

/// Runs a PowerShell script hidden; Ok(stdout) or Err(stderr).
/// The script is passed as one argument, so it must not contain double quotes.
pub fn ps(script: &str) -> Result<String, String> {
    // This runs elevated. PowerShell is started by full path, and the folders it finds programs
    // and modules in are pinned to Windows' own, so nothing a user-level program planted or set
    // in the environment can be picked up and run as administrator.
    let (windows, system) = crate::sys::windows_dir()
        .zip(crate::sys::system_dir())
        .ok_or("Windows system folder unavailable")?;
    let home = system.join(r"WindowsPowerShell\v1.0");
    let out = Command::new(home.join("powershell.exe"))
        .env("SystemRoot", &windows)
        .env("PATH", format!("{0};{0}\\Wbem;{1}", system.display(), home.display()))
        .env("PSModulePath", home.join("Modules"))
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command"])
        .arg(format!("[Console]::OutputEncoding=[System.Text.UTF8Encoding]::new($false);$ErrorActionPreference='Stop';{script}"))
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

fn root(hive: Hive) -> RegKey {
    RegKey::predef(match hive {
        Hive::Hkcu => HKEY_CURRENT_USER,
        Hive::Hklm => HKEY_LOCAL_MACHINE,
    })
}

fn matches(hive: Hive, path: &str, name: &str, want: &Val) -> bool {
    let Ok(key) = root(hive).open_subkey(path) else {
        return matches!(want, Val::Delete);
    };
    match want {
        Val::Dword(d) => key.get_value::<u32, _>(name).is_ok_and(|v| v == *d),
        Val::Str(s) => key.get_value::<String, _>(name).is_ok_and(|v| v == *s),
        Val::Bin(b) => key.get_raw_value(name).is_ok_and(|v| v.bytes[..] == **b),
        Val::Delete => key.get_raw_value(name).is_err(),
    }
}

fn write(hive: Hive, path: &str, name: &str, val: &Val) -> io::Result<()> {
    let missing_ok = |r: io::Result<()>| match r {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        r => r,
    };
    match val {
        Val::Delete => missing_ok(
            root(hive)
                .open_subkey_with_flags(path, KEY_SET_VALUE)
                .and_then(|k| k.delete_value(name)),
        ),
        Val::Dword(d) => root(hive).create_subkey(path)?.0.set_value(name, d),
        Val::Str(s) => root(hive).create_subkey(path)?.0.set_value(name, s),
        Val::Bin(b) => root(hive).create_subkey(path)?.0.set_raw_value(
            name,
            &RegValue {
                bytes: b.to_vec().into(),
                vtype: REG_BINARY,
            },
        ),
    }
}

pub fn is_on(actions: &[Action]) -> bool {
    actions.iter().all(|a| match a {
        Action::Reg {
            hive,
            path,
            name,
            on,
            ..
        } => matches(*hive, path, name, on),
        Action::Script { check, .. } => ps(check).is_ok_and(|s| s.eq_ignore_ascii_case("true")),
    })
}

pub fn apply(actions: &[Action], turn_on: bool) -> Result<(), String> {
    for a in actions {
        match a {
            Action::Reg {
                hive,
                path,
                name,
                on,
                off,
            } => write(*hive, path, name, if turn_on { on } else { off })
                .map_err(|e| format!("{path}\\{name}: {e}"))?,
            Action::Script { apply, undo, .. } => {
                ps(if turn_on { apply } else { undo })?;
            }
        }
    }
    Ok(())
}

/// One human-readable line per action, shown under "What changes".
pub fn describe(a: &Action) -> String {
    match a {
        Action::Reg {
            hive,
            path,
            name,
            on,
            ..
        } => {
            let hive = match hive {
                Hive::Hkcu => "HKCU",
                Hive::Hklm => "HKLM",
            };
            let val = match on {
                Val::Dword(d) => d.to_string(),
                Val::Str(s) => format!("\"{s}\""),
                Val::Bin(b) => b
                    .iter()
                    .map(|x| format!("{x:02X}"))
                    .collect::<Vec<_>>()
                    .join(" "),
                Val::Delete => "(removed)".into(),
            };
            format!("{hive}\\{path} : {name} = {val}")
        }
        Action::Script { apply, .. } => format!("PowerShell: {apply}"),
    }
}

/// Raw value of a registry action, for the live test to put back afterwards.
#[cfg(test)]
pub fn snapshot(a: &Action) -> Option<RegValue<'static>> {
    let Action::Reg {
        hive, path, name, ..
    } = a
    else {
        return None;
    };
    root(*hive)
        .open_subkey(path)
        .ok()?
        .get_raw_value(name)
        .ok()
        .map(|v| RegValue {
            bytes: v.bytes.into_owned().into(),
            vtype: v.vtype,
        })
}

#[cfg(test)]
pub fn restore(a: &Action, saved: Option<RegValue<'static>>) {
    let Action::Reg {
        hive, path, name, ..
    } = a
    else {
        return;
    };
    let _ = match saved {
        Some(v) => root(*hive)
            .create_subkey(path)
            .and_then(|(k, _)| k.set_raw_value(name, &v)),
        None => write(*hive, path, name, &Val::Delete),
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Elevated PowerShell must not load commands from folders the user can write to.
    #[test]
    fn powershell_only_loads_modules_from_protected_folders() {
        let saved = std::env::var_os("PSModulePath");
        std::env::set_var("PSModulePath", r"C:\Users\Public\opticlean-decoy-modules");
        let seen = ps("$env:PSModulePath");
        match saved {
            Some(v) => std::env::set_var("PSModulePath", v),
            None => std::env::remove_var("PSModulePath"),
        }
        let seen = seen.unwrap().to_lowercase();
        assert!(!seen.contains("decoy"), "user-chosen module folder was used: {seen}");
        assert!(!seen.contains(r"\users\"), "module folder inside a user profile: {seen}");
    }

    #[test]
    #[ignore = "host registry integration test; requires an unrestricted Windows test account"]
    fn registry_round_trip() {
        const PATH: &str = r"Software\OptiCleanSelfTest";
        const ACTIONS: &[Action] = &[
            Action::Reg {
                hive: Hive::Hkcu,
                path: PATH,
                name: "d",
                on: Val::Dword(1),
                off: Val::Delete,
            },
            Action::Reg {
                hive: Hive::Hkcu,
                path: PATH,
                name: "s",
                on: Val::Str("a"),
                off: Val::Str("b"),
            },
            Action::Reg {
                hive: Hive::Hkcu,
                path: PATH,
                name: "b",
                on: Val::Bin(&[1, 2]),
                off: Val::Delete,
            },
        ];
        assert!(!is_on(ACTIONS));
        apply(ACTIONS, true).unwrap();
        assert!(is_on(ACTIONS));
        apply(ACTIONS, false).unwrap();
        assert!(!is_on(ACTIONS));
        let key = root(Hive::Hkcu).open_subkey(PATH).unwrap();
        assert_eq!(key.get_value::<String, _>("s").unwrap(), "b");
        assert!(key.get_raw_value("d").is_err());
        apply(ACTIONS, false).unwrap(); // undoing twice is not an error
        root(Hive::Hkcu).delete_subkey(PATH).unwrap();
    }
}
