use serde::Serialize;
use winreg::{enums::*, RegKey, RegValue, HKEY};

const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const APPROVED: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved";
const FOLDER: &str = r"Microsoft\Windows\Start Menu\Programs\Startup";

/// Where startup entries live, and the StartupApproved subkey Task Manager uses to switch them.
struct Source {
    label: &'static str,
    hive: HKEY,
    run: Option<&'static str>,     // registry Run key
    dir_env: Option<&'static str>, // or a Startup folder under this env var
    approved: &'static str,
}

const SOURCES: &[Source] = &[
    Source {
        label: "Current user",
        hive: HKEY_CURRENT_USER,
        run: Some(RUN),
        dir_env: None,
        approved: "Run",
    },
    Source {
        label: "All users",
        hive: HKEY_LOCAL_MACHINE,
        run: Some(RUN),
        dir_env: None,
        approved: "Run",
    },
    Source {
        label: "All users (32-bit)",
        hive: HKEY_LOCAL_MACHINE,
        run: Some(r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Run"),
        dir_env: None,
        approved: "Run32",
    },
    Source {
        label: "Startup folder",
        hive: HKEY_CURRENT_USER,
        run: None,
        dir_env: Some("APPDATA"),
        approved: "StartupFolder",
    },
    Source {
        label: "Startup folder (all users)",
        hive: HKEY_LOCAL_MACHINE,
        run: None,
        dir_env: Some("ProgramData"),
        approved: "StartupFolder",
    },
];

#[derive(Serialize)]
pub struct Item {
    name: String,
    command: String,
    source: &'static str,
    enabled: bool,
}

pub fn list() -> Vec<Item> {
    let mut items = Vec::new();
    for s in SOURCES {
        let root = RegKey::predef(s.hive);
        let approved = root.open_subkey(format!(r"{APPROVED}\{}", s.approved)).ok();
        // Missing flag = enabled; otherwise an even first byte (02/06) means enabled.
        let enabled = |name: &str| {
            approved
                .as_ref()
                .and_then(|k| k.get_raw_value(name).ok())
                .and_then(|v| v.bytes.first().copied())
                .is_none_or(|b| b % 2 == 0)
        };
        let mut entries: Vec<(String, String)> = Vec::new();
        if let Some(run) = s.run {
            if let Ok(key) = root.open_subkey(run) {
                entries.extend(key.enum_values().flatten().map(|(n, v)| (n, v.to_string())));
            }
        } else if let Some(dir) = s.dir_env.and_then(std::env::var_os) {
            let dir = std::path::Path::new(&dir).join(FOLDER);
            for f in std::fs::read_dir(dir).into_iter().flatten().flatten() {
                let name = f.file_name().to_string_lossy().into_owned();
                if name != "desktop.ini" {
                    entries.push((name, f.path().to_string_lossy().into_owned()));
                }
            }
        }
        for (name, command) in entries {
            if !name.is_empty() {
                items.push(Item {
                    enabled: enabled(&name),
                    name,
                    command,
                    source: s.label,
                });
            }
        }
    }
    items
}

fn is_listed(items: &[Item], source: &str, name: &str) -> bool {
    items.iter().any(|i| i.source == source && i.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_entries_that_exist_can_be_switched() {
        let items = [Item {
            name: "Discord".into(),
            command: String::new(),
            source: "Current user",
            enabled: true,
        }];
        assert!(is_listed(&items, "Current user", "Discord"));
        assert!(!is_listed(&items, "Current user", "NotThere"));
        assert!(!is_listed(&items, "All users", "Discord"));
    }
}

pub fn set(source: &str, name: &str, enabled: bool) -> Result<(), String> {
    let s = SOURCES
        .iter()
        .find(|s| s.label == source)
        .ok_or("unknown startup source")?;
    // Only switch entries that exist, so this cannot be used to write arbitrary value names.
    if !is_listed(&list(), source, name) {
        return Err(format!("{name}: no such startup entry"));
    }
    let mut bytes = vec![0u8; 12];
    bytes[0] = if enabled { 2 } else { 3 };
    RegKey::predef(s.hive)
        .create_subkey(format!(r"{APPROVED}\{}", s.approved))
        .and_then(|(k, _)| {
            k.set_raw_value(
                name,
                &RegValue {
                    bytes: bytes.into(),
                    vtype: REG_BINARY,
                },
            )
        })
        .map_err(|e| format!("{name}: {e}"))
}
