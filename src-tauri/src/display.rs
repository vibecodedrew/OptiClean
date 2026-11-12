//! Read-only display-mode checks at the moment a user adjusts colors. No polling.
use windows::Win32::{
    Devices::Display::*,
    Foundation::{ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS},
};

// Windows 11's newer query distinguishes HDR from automatic color management.
// This SDK structure is not yet exposed by windows 0.62.2.
#[repr(C)]
#[derive(Default)]
struct AdvancedColorInfo2 {
    header: DISPLAYCONFIG_DEVICE_INFO_HEADER,
    value: u32,
    encoding: i32,
    bits_per_channel: u32,
    active_mode: i32,
}

fn check_mode(mode: i32) -> Result<(), String> {
    match mode {
        0 => Ok(()),
        1 => Err("Windows advanced color management is active. These output controls need an SDR display with advanced color off; no colors were changed.".into()),
        2 => Err("Windows HDR is active on the primary display. Turn HDR off to use these SDR color controls; no colors were changed.".into()),
        _ => Err("Windows reported an unknown display color mode; no colors were changed.".into()),
    }
}

fn check_target(path: &DISPLAYCONFIG_PATH_INFO) -> Result<(), String> {
    unsafe {
        let mut modern = AdvancedColorInfo2 {
            header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                r#type: DISPLAYCONFIG_DEVICE_INFO_TYPE(15),
                size: std::mem::size_of::<AdvancedColorInfo2>() as u32,
                adapterId: path.targetInfo.adapterId,
                id: path.targetInfo.id,
            },
            ..Default::default()
        };
        let result = DisplayConfigGetDeviceInfo(&mut modern.header);
        if result == 0 {
            return check_mode(modern.active_mode);
        }
        // Older Windows builds do not implement GET_ADVANCED_COLOR_INFO_2.
        if ![50, 87, 120].contains(&result) {
            return Err(format!("Could not check the display color mode (Windows {result}); no colors were changed."));
        }
        let mut legacy = DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO {
            header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                r#type: DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO,
                size: std::mem::size_of::<DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO>() as u32,
                adapterId: path.targetInfo.adapterId,
                id: path.targetInfo.id,
            },
            ..Default::default()
        };
        let result = DisplayConfigGetDeviceInfo(&mut legacy.header);
        if result != 0 {
            return Err(format!(
                "Could not check HDR status (Windows {result}); no colors were changed."
            ));
        }
        if legacy.Anonymous.value & 2 != 0 {
            Err("Windows HDR or advanced color management is active. These output controls need SDR mode; no colors were changed.".into())
        } else {
            Ok(())
        }
    }
}

pub fn ensure_sdr(primary_name: &str) -> Result<(), String> {
    unsafe {
        // Display changes can invalidate buffer sizes between these two calls.
        for _ in 0..3 {
            let (mut path_count, mut mode_count) = (0, 0);
            let result = GetDisplayConfigBufferSizes(
                QDC_ONLY_ACTIVE_PATHS,
                &mut path_count,
                &mut mode_count,
            );
            if result != ERROR_SUCCESS {
                return Err(format!(
                    "Could not inspect display mode (Windows {}); no colors were changed.",
                    result.0
                ));
            }
            if path_count == 0 || path_count > 1024 || mode_count > 16384 {
                return Err(
                    "Windows returned an invalid display topology; no colors were changed.".into(),
                );
            }
            let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
            let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); mode_count as usize];
            let result = QueryDisplayConfig(
                QDC_ONLY_ACTIVE_PATHS,
                &mut path_count,
                paths.as_mut_ptr(),
                &mut mode_count,
                modes.as_mut_ptr(),
                None,
            );
            if result == ERROR_INSUFFICIENT_BUFFER {
                continue;
            }
            if result != ERROR_SUCCESS {
                return Err(format!(
                    "Could not read display mode (Windows {}); no colors were changed.",
                    result.0
                ));
            }
            let mut found = false;
            for path in &paths[..path_count as usize] {
                let mut source = DISPLAYCONFIG_SOURCE_DEVICE_NAME {
                    header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                        r#type: DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
                        size: std::mem::size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32,
                        adapterId: path.sourceInfo.adapterId,
                        id: path.sourceInfo.id,
                    },
                    ..Default::default()
                };
                if DisplayConfigGetDeviceInfo(&mut source.header) != 0 {
                    continue;
                }
                let end = source
                    .viewGdiDeviceName
                    .iter()
                    .position(|c| *c == 0)
                    .unwrap_or(source.viewGdiDeviceName.len());
                let name = String::from_utf16_lossy(&source.viewGdiDeviceName[..end]);
                if name.eq_ignore_ascii_case(primary_name) {
                    found = true;
                    // Check every clone target sharing this source, not another monitor.
                    check_target(path)?;
                }
            }
            return if found {
                Ok(())
            } else {
                Err("Could not match the primary display to its color mode; no colors were changed.".into())
            };
        }
    }
    Err("The display configuration is changing. Try adjusting the color slider again.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_known_sdr_modes_allow_output_changes() {
        assert!(check_mode(0).is_ok());
        assert!(check_mode(1).unwrap_err().contains("advanced color"));
        assert!(check_mode(2).unwrap_err().contains("HDR"));
        assert!(check_mode(-1).is_err());
        assert!(check_mode(3).is_err());
        assert_eq!(std::mem::size_of::<AdvancedColorInfo2>(), 36);
    }
}
