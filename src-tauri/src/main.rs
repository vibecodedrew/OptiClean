#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod apps;
mod clean;
mod color;
mod display;
mod recovery;
mod reg;
mod startup;
mod sys;
mod tweaks;

use serde::Serialize;
use tweaks::{Tweak, TWEAKS};

#[derive(Serialize)]
struct TweakDto {
    #[serde(flatten)]
    tweak: &'static Tweak,
    changes: Vec<String>,
    enabled: bool,
}

fn find(id: &str) -> Result<&'static Tweak, String> {
    TWEAKS
        .iter()
        .find(|t| t.id == id)
        .ok_or_else(|| format!("unknown tweak {id}"))
}

#[tauri::command(async)]
fn list_tweaks() -> Vec<TweakDto> {
    TWEAKS
        .iter()
        .map(|t| TweakDto {
            tweak: t,
            changes: t.actions.iter().map(reg::describe).collect(),
            enabled: reg::is_on(t.actions),
        })
        .collect()
}

// `async` here moves these blocking calls off the UI thread.
#[tauri::command(async)]
fn tweak_status(id: String) -> Result<bool, String> {
    Ok(reg::is_on(find(&id)?.actions))
}

/// Applies or undoes, then returns the state actually read back from Windows.
#[tauri::command(async)]
fn set_tweak(id: String, on: bool) -> Result<bool, String> {
    let t = find(&id)?;
    recovery::ensure()?;
    reg::apply(t.actions, on)?;
    Ok(reg::is_on(t.actions))
}

#[tauri::command(async)]
fn restore_point() -> Result<(), String> {
    recovery::ensure()
}

#[tauri::command(async)]
fn recovery_status() -> Result<String, String> {
    recovery::status()
}

#[tauri::command(async)]
fn open_recovery() -> Result<(), String> {
    recovery::open()
}

#[tauri::command(async)]
fn list_startup() -> Vec<startup::Item> {
    startup::list()
}

#[tauri::command(async)]
fn set_startup(source: String, name: String, enabled: bool) -> Result<(), String> {
    recovery::ensure()?;
    startup::set(&source, &name, enabled)
}

#[tauri::command(async)]
fn list_apps() -> Result<Vec<apps::App>, String> {
    apps::list()
}

#[tauri::command(async)]
fn remove_apps(pkgs: Vec<String>) -> Result<Vec<String>, String> {
    recovery::ensure()?;
    Ok(apps::remove(&pkgs))
}

#[tauri::command(async)]
fn list_cleaners() -> Vec<clean::Info> {
    clean::list()
}

#[tauri::command(async)]
fn run_cleaner(id: String) -> Result<u64, String> {
    recovery::ensure()?;
    clean::clean(&id)
}

#[tauri::command(async)]
fn set_color(saturation: f32, shadow: f32) -> Result<(), String> {
    color::apply(saturation, shadow)
}

#[tauri::command(async)]
fn color_state() -> Option<(f32, f32)> {
    color::saved()
}

#[tauri::command(async)]
fn reset_color() -> Result<(), String> {
    color::reset()
}

#[tauri::command(async)]
fn clear_legacy_color() -> Result<(), String> {
    color::clear_legacy()
}

/// One compact JSON object; the frontend parses it.
#[tauri::command(async)]
fn hardware() -> Result<String, String> {
    reg::ps(
        r"function Read-Cim($class){ try { @(Get-CimInstance $class -ErrorAction Stop) } catch { @() } };
$c=Read-Cim Win32_Processor | Select-Object -First 1;
$gs=@(Read-Cim Win32_VideoController | ForEach-Object Name);
$b=Read-Cim Win32_BaseBoard | Select-Object -First 1;
$m=@(Read-Cim Win32_PhysicalMemory);
$bios=Read-Cim Win32_BIOS | Select-Object -First 1;
$system=Read-Cim Win32_ComputerSystem | Select-Object -First 1;
$psu=Read-Cim CIM_PowerSupply | Select-Object -First 1;
[pscustomobject]@{cpu=([string]$c.Name).Trim();gpus=$gs;board=([string]$b.Manufacturer+' '+[string]$b.Product).Trim();
bios=[string]$bios.SMBIOSBIOSVersion;ramGb=[int][math]::Round(($m | Measure-Object Capacity -Sum).Sum/1GB);
ramSpeed=[int]($m | Measure-Object ConfiguredClockSpeed -Maximum).Maximum;
portable=([int]$system.PCSystemType -eq 2);
psu=[string]$psu.Name} | ConvertTo-Json -Compress",
    )
}

#[tauri::command]
fn app_ready(window: tauri::WebviewWindow) -> Result<(), String> {
    window.show().map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            use tauri::Manager;
            use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Settings3;
            use windows_core::Interface;
            if let Some(window) = app.get_webview_window("main") {
                window.with_webview(|webview| unsafe {
                    // Disable WebView2 context menu and print/browser accelerator shortcuts.
                    if let Ok(settings) = webview
                        .controller()
                        .CoreWebView2()
                        .and_then(|w| w.Settings())
                    {
                        let _ = settings.SetAreDefaultContextMenusEnabled(false);
                        if let Ok(settings3) = settings.cast::<ICoreWebView2Settings3>() {
                            let _ = settings3.SetAreBrowserAcceleratorKeysEnabled(false);
                        }
                    }
                })?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_ready,
            set_color,
            color_state,
            hardware,
            reset_color,
            clear_legacy_color,
            recovery_status,
            open_recovery,
            list_tweaks,
            tweak_status,
            set_tweak,
            restore_point,
            list_startup,
            set_startup,
            list_apps,
            remove_apps,
            list_cleaners,
            run_cleaner
        ])
        .run(tauri::generate_context!())
        .expect("error while running OptiClean");
}

