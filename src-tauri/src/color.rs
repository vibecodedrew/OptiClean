//! On-demand NVIDIA output matrix, never a game overlay or a per-frame filter.
//! Extended saturation may clip. Shadow lift composes with the saved calibration.
//! No output setting is written at application startup.
use serde::{Deserialize, Serialize};
use std::{
    ffi::{c_void, CString},
    mem::transmute,
    sync::Mutex,
};

type Query = unsafe extern "C" fn(u32) -> *const c_void;
#[link(name = "kernel32")]
extern "system" {
    fn LoadLibraryExW(name: *const u16, file: isize, flags: u32) -> isize;
    fn FreeLibrary(module: isize) -> i32;
    fn GetProcAddress(module: isize, name: *const u8) -> Option<Query>;
}
const INITIALIZE: u32 = 0x0150_E828;
const PRIMARY: u32 = 0x1E9D_8A31;
const GET_CSC: u32 = 0x8159_E87A;
const SET_CSC: u32 = 0xFCAB_D23A;
const VERSION: u32 = 0x1007C;
static LOCK: Mutex<()> = Mutex::new(());

#[repr(C)]
struct DisplayDevice {
    size: u32,
    name: [u16; 32],
    description: [u16; 128],
    flags: u32,
    id: [u16; 128],
    key: [u16; 128],
}

fn primary_monitor() -> Result<(String, String), String> {
    #[link(name = "user32")]
    extern "system" {
        fn EnumDisplayDevicesW(
            name: *const u16,
            index: u32,
            device: *mut DisplayDevice,
            flags: u32,
        ) -> i32;
    }
    fn value(s: &[u16]) -> String {
        String::from_utf16_lossy(&s[..s.iter().position(|x| *x == 0).unwrap_or(s.len())])
    }
    unsafe {
        for index in 0..64 {
            let mut adapter: DisplayDevice = std::mem::zeroed();
            adapter.size = std::mem::size_of::<DisplayDevice>() as u32;
            if EnumDisplayDevicesW(std::ptr::null(), index, &mut adapter, 0) == 0 {
                break;
            }
            if adapter.flags & 4 != 0 {
                let mut monitor: DisplayDevice = std::mem::zeroed();
                monitor.size = std::mem::size_of::<DisplayDevice>() as u32;
                if EnumDisplayDevicesW(adapter.name.as_ptr(), 0, &mut monitor, 1) != 0
                    && monitor.id[0] != 0
                {
                    return Ok((
                        value(&adapter.name),
                        format!("{}|{}", value(&adapter.id), value(&monitor.id)),
                    ));
                }
            }
        }
    }
    Err("Could not identify the primary monitor. No colors were changed.".into())
}

#[repr(C)]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct Csc {
    version: u32,
    content_space: u32,
    monitor_space: u32,
    unknown: [u32; 2],
    use_matrix1: u32,
    matrix1: [f32; 12],
    use_matrix2: u32,
    matrix2: [f32; 12],
}

struct Driver {
    lib: isize,
    query: Query,
}
impl Driver {
    fn open() -> Result<Self, String> {
        unsafe {
            // Search only System32, never the working directory of this elevated app.
            let name: Vec<u16> = "nvapi64.dll\0".encode_utf16().collect();
            let lib = LoadLibraryExW(name.as_ptr(), 0, 0x800);
            if lib == 0 {
                return Err("NVIDIA display driver unavailable. For AMD/Intel or hybrid laptop displays, use the display vendor's color controls.".into());
            }
            let Some(query) = GetProcAddress(lib, c"nvapi_QueryInterface".as_ptr().cast()) else {
                FreeLibrary(lib);
                return Err("NVIDIA interface unavailable".into());
            };
            let driver = Self { lib, query };
            let init: unsafe extern "C" fn() -> i32 = transmute(driver.get(INITIALIZE)?);
            check(init(), "Initialize")?;
            Ok(driver)
        }
    }
    fn get(&self, id: u32) -> Result<*const c_void, String> {
        let p = unsafe { (self.query)(id) };
        if p.is_null() {
            Err(format!("Driver does not support color interface {id:#x}"))
        } else {
            Ok(p)
        }
    }
    fn primary(&self) -> Result<u32, String> {
        unsafe {
            let f: unsafe extern "C" fn(*mut u32) -> i32 = transmute(self.get(PRIMARY)?);
            let mut id = 0;
            check(f(&mut id), "Find primary display")?;
            Ok(id)
        }
    }
    fn read(&self, display: u32) -> Result<Csc, String> {
        unsafe {
            let f: unsafe extern "C" fn(u32, *mut Csc) -> i32 = transmute(self.get(GET_CSC)?);
            let mut c: Csc = std::mem::zeroed();
            c.version = VERSION;
            check(f(display, &mut c), "Read original color settings")?;
            Ok(c)
        }
    }
    fn write(&self, display: u32, c: &Csc) -> Result<(), String> {
        unsafe {
            let f: unsafe extern "C" fn(u32, *mut Csc) -> i32 = transmute(self.get(SET_CSC)?);
            check(f(display, &mut c.clone()), "Apply color settings")
        }
    }
    fn display_handle(&self, name: &str) -> Result<*mut c_void, String> {
        unsafe {
            let f: unsafe extern "C" fn(*const i8, *mut *mut c_void) -> i32 =
                transmute(self.get(0x35C29134)?);
            let name = CString::new(name).map_err(|e| e.to_string())?;
            let mut handle = std::ptr::null_mut();
            check(f(name.as_ptr(), &mut handle), "Match NVIDIA display")?;
            if handle.is_null() {
                return Err("No NVIDIA handle for the primary display".into());
            }
            Ok(handle)
        }
    }
    fn read_dvc(&self, handle: *mut c_void) -> Result<Dvc, String> {
        unsafe {
            let f: unsafe extern "C" fn(*mut c_void, u32, *mut Dvc) -> i32 =
                transmute(self.get(0x0E45002D)?);
            let mut d = Dvc {
                version: 0x10014,
                current: 0,
                minimum: 0,
                maximum: 0,
                neutral: 0,
            };
            check(f(handle, 0, &mut d), "Read NVIDIA Digital Vibrance")?;
            validate_dvc(&d)?;
            Ok(d)
        }
    }
    fn write_dvc(&self, handle: *mut c_void, level: i32) -> Result<(), String> {
        let range = self.read_dvc(handle)?;
        if !(range.minimum..=range.maximum).contains(&level) {
            return Err("Saved NVIDIA vibrance is outside this driver's range".into());
        }
        unsafe {
            let f: unsafe extern "C" fn(*mut c_void, u32, *mut Dvc) -> i32 =
                transmute(self.get(0x4A82C2B1)?);
            let mut d = Dvc {
                version: 0x10014,
                current: level,
                minimum: 0,
                maximum: 0,
                neutral: 0,
            };
            check(f(handle, 0, &mut d), "Set NVIDIA Digital Vibrance")?;
        }
        if self.read_dvc(handle)?.current != level {
            return Err("NVIDIA did not retain its Digital Vibrance setting".into());
        }
        Ok(())
    }
}
impl Drop for Driver {
    fn drop(&mut self) {
        unsafe {
            let p = (self.query)(0xD22BDD7E); // NvAPI_Unload
            if !p.is_null() {
                let unload: unsafe extern "C" fn() -> i32 = transmute(p);
                unload();
            }
            FreeLibrary(self.lib);
        }
    }
}
fn check(status: i32, action: &str) -> Result<(), String> {
    if status == 0 {
        Ok(())
    } else {
        Err(format!("{action}: NVIDIA error {status}. Extended color needs a supported NVIDIA-connected primary SDR display; use the vendor controls for HDR."))
    }
}

fn matrix(s: f32) -> [f32; 12] {
    let w = [0.2126, 0.7152, 0.0722];
    let mut m = [0.; 12];
    for i in 0..3 {
        for j in 0..3 {
            m[i * 4 + j] = w[j] * (1. - s) + if i == j { s } else { 0. };
        }
    }
    m
}

#[repr(C)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Dvc {
    version: u32,
    current: i32,
    minimum: i32,
    maximum: i32,
    neutral: i32,
}
fn validate_dvc(d: &Dvc) -> Result<(), String> {
    if d.version != 0x10014
        || d.minimum >= d.maximum
        || !(d.minimum..=d.maximum).contains(&d.current)
        || !(d.minimum..=d.maximum).contains(&d.neutral)
    {
        Err("Invalid NVIDIA Digital Vibrance range".into())
    } else {
        Ok(())
    }
}

type Ramp = [[u16; 256]; 3];
struct GammaDc(isize);
#[link(name = "gdi32")]
extern "system" {
    fn CreateDCW(
        driver: *const u16,
        device: *const u16,
        output: *const u16,
        init: *const c_void,
    ) -> isize;
    fn DeleteDC(dc: isize) -> i32;
    fn GetDeviceGammaRamp(dc: isize, ramp: *mut Ramp) -> i32;
    fn SetDeviceGammaRamp(dc: isize, ramp: *const Ramp) -> i32;
}
impl GammaDc {
    fn open(name: &str) -> Result<Self, String> {
        let driver: Vec<u16> = "DISPLAY\0".encode_utf16().collect();
        let name: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
        let dc = unsafe {
            CreateDCW(
                driver.as_ptr(),
                name.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
            )
        };
        if dc == 0 {
            Err("Could not access primary-display gamma".into())
        } else {
            Ok(Self(dc))
        }
    }
    fn read(&self) -> Result<Ramp, String> {
        let mut ramp = [[0; 256]; 3];
        if unsafe { GetDeviceGammaRamp(self.0, &mut ramp) } == 0 {
            Err("Display cannot report its gamma ramp".into())
        } else {
            Ok(ramp)
        }
    }
    fn write(&self, ramp: &Ramp) -> Result<(), String> {
        if unsafe { SetDeviceGammaRamp(self.0, ramp) } == 0 {
            return Err("Display refused shadow boost. Check that HDR/Auto HDR is off.".into());
        }
        // Windows may return success without applying a ramp. Always verify readback.
        if !ramps_close(&self.read()?, ramp) {
            return Err("Display ignored shadow boost. No confirmed change was made.".into());
        }
        Ok(())
    }
}
impl Drop for GammaDc {
    fn drop(&mut self) {
        unsafe {
            DeleteDC(self.0);
        }
    }
}
fn ramps_close(a: &Ramp, b: &Ramp) -> bool {
    a.iter()
        .flatten()
        .zip(b.iter().flatten())
        .all(|(x, y)| x.abs_diff(*y) <= 128)
}
fn linear_ramp() -> Ramp {
    std::array::from_fn(|_| std::array::from_fn(|i| (i * 257) as u16))
}
fn sample(channel: &[u16; 256], x: f32) -> u16 {
    let at = x.clamp(0., 1.) * 255.;
    let lo = at.floor() as usize;
    let hi = (lo + 1).min(255);
    (channel[lo] as f32 + (channel[hi] as f32 - channel[lo] as f32) * (at - lo as f32)).round()
        as u16
}
fn shadow_ramp(base: &Ramp, strength: f32) -> Ramp {
    if strength == 1. {
        return *base;
    }
    std::array::from_fn(|channel| {
        std::array::from_fn(|i| {
            let x = i as f32 / 255.;
            // Finite slope at black (no grey haze) and slope >= 1 - k/3 everywhere,
            // so texture contrast is never flattened. Highlights are untouched.
            sample(&base[channel], x + (strength - 1.) * x * (1. - x) * (1. - x))
        })
    })
}
/// Gentle S-curve that grows with vibrance: mid-tone slope up to 1.125 for punch,
/// end slopes never below 0.75 so shadow and highlight texture survive.
// shortcut: strength is tied to vibrance, give it its own slider if users want to tune it.
fn pop_ramp(base: &Ramp, saturation: f32) -> Ramp {
    let c = 0.25 * (saturation - 1.).min(1.);
    if c == 0. {
        return *base;
    }
    std::array::from_fn(|channel| {
        std::array::from_fn(|i| {
            let x = i as f32 / 255.;
            sample(&base[channel], x + c * x * (1. - x) * (2. * x - 1.))
        })
    })
}
/// The matrix is scaled down by this so boosted reds/oranges (red output up to
/// 1.25) reach the tone curve instead of being clipped flat by the driver.
fn headroom(saturation: f32) -> f32 {
    1. + ((saturation - 1.) * 0.3).min(0.25)
}
/// Undoes the headroom scale: exact below the knee, then a soft shoulder that
/// compresses the over-range values into the top of the range, keeping texture.
/// Cost: white lands at ~94% signal at full headroom.
fn knee_ramp(base: &Ramp, headroom: f32) -> Ramp {
    if headroom == 1. {
        return *base;
    }
    const KNEE: f32 = 0.75;
    let a = (headroom - KNEE) / (1. - KNEE);
    std::array::from_fn(|channel| {
        std::array::from_fn(|i| {
            let x = i as f32 / 255. * headroom;
            let y = if x <= KNEE {
                x
            } else {
                // Slope 1 at the knee (no visible seam), reaches 1.0 at x = headroom.
                KNEE + (1. - KNEE) * (1. - (1. - (x - KNEE) / (headroom - KNEE)).max(0.).powf(a))
            };
            sample(&base[channel], y)
        })
    })
}
fn neutral_csc() -> Csc {
    Csc {
        version: VERSION,
        content_space: 2,
        monitor_space: 0,
        unknown: [0; 2],
        use_matrix1: 0,
        matrix1: [0.; 12],
        use_matrix2: 0,
        matrix2: [0.; 12],
    }
}
fn desired_csc(saturation: f32) -> Csc {
    if saturation == 1. {
        return neutral_csc();
    }
    // Above 160% the green output grows at 35% speed: sunlit grass/terrain is
    // green-dominant and otherwise clips to a flat patch. Reds and blues get the full boost.
    // The matrix offset column must stay 0: this driver blows the image out if it is used.
    let mut m = matrix(saturation);
    let green = matrix(saturation.min(1.6 + (saturation - 1.6).max(0.) * 0.35));
    m[4..8].copy_from_slice(&green[4..8]);
    let h = headroom(saturation);
    m.iter_mut().for_each(|v| *v /= h); // knee_ramp restores brightness after the driver's clamp
    Csc {
        monitor_space: 2,
        use_matrix1: 1,
        matrix1: m,
        ..neutral_csc()
    }
}
fn csc_matches(a: &Csc, b: &Csc) -> bool {
    a.use_matrix1 == b.use_matrix1
        && a.use_matrix2 == b.use_matrix2
        && (b.use_matrix1 == 0
            || a.matrix1
                .iter()
                .zip(b.matrix1)
                .all(|(x, y)| (*x - y).abs() < 0.0001))
        && (b.use_matrix2 == 0
            || a.matrix2
                .iter()
                .zip(b.matrix2)
                .all(|(x, y)| (*x - y).abs() < 0.0001))
}

#[derive(Clone)]
struct Snapshot {
    settings: Csc,
    dvc: Dvc,
    gamma: Ramp,
}
trait Output {
    fn csc(&mut self, value: &Csc) -> Result<(), String>;
    fn dvc(&mut self, level: i32) -> Result<(), String>;
    fn gamma(&mut self, value: &Ramp) -> Result<(), String>;
}
struct LiveOutput<'a> {
    driver: &'a Driver,
    display: u32,
    handle: *mut c_void,
    gamma: &'a GammaDc,
}
impl Output for LiveOutput<'_> {
    fn csc(&mut self, value: &Csc) -> Result<(), String> {
        self.driver.write(self.display, value)?;
        if csc_matches(&self.driver.read(self.display)?, value) {
            Ok(())
        } else {
            Err("Driver did not retain OptiClean's color matrix".into())
        }
    }
    fn dvc(&mut self, level: i32) -> Result<(), String> {
        self.driver.write_dvc(self.handle, level)
    }
    fn gamma(&mut self, value: &Ramp) -> Result<(), String> {
        // A driver color-setting change can reset the ramp. Recheck after CSC/DVC,
        // but avoid a gamma write when the saved curve is already in effect.
        if self.gamma.read()? == *value {
            Ok(())
        } else {
            self.gamma.write(value)
        }
    }
}
fn rollback(device: &mut impl Output, previous: &Snapshot) -> Result<(), String> {
    // Attempt every stage even if one restoration fails.
    let mut errors = Vec::new();
    if let Err(e) = device.csc(&previous.settings) {
        errors.push(e);
    }
    if let Err(e) = device.dvc(previous.dvc.current) {
        errors.push(e);
    }
    if let Err(e) = device.gamma(&previous.gamma) {
        errors.push(e);
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}
fn apply_transaction(
    device: &mut impl Output,
    previous: &Snapshot,
    target: &Csc,
    gamma: &Ramp,
) -> Result<(), String> {
    let result = (|| {
        // Neutral means the driver's default (usually 50%), never minimum/grayscale.
        if previous.dvc.current != previous.dvc.neutral {
            device.dvc(previous.dvc.neutral)?;
        }
        device.csc(target)?;
        device.gamma(gamma)?;
        Ok(())
    })();
    if let Err(e) = result {
        return match rollback(device, previous) {
            Ok(()) => Err(e),
            Err(r) => Err(format!("{e}; recovery also failed: {r}")),
        };
    }
    Ok(())
}

#[derive(Serialize, Deserialize)]
struct Baseline {
    display: u32,
    monitor: String,
    settings: Csc,
    #[serde(default)]
    dvc: Option<Dvc>,
    #[serde(default)]
    gamma: Option<Vec<Vec<u16>>>,
}
fn data_dir() -> Result<std::path::PathBuf, String> {
    let dir = std::path::PathBuf::from(
        std::env::var_os("LOCALAPPDATA").ok_or("Local app data unavailable")?,
    )
    .join("OptiClean");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}
fn baseline_path(display: u32) -> Result<std::path::PathBuf, String> {
    Ok(data_dir()?.join(format!("color-baseline-{display}.json")))
}
/// The slider values last applied, so a reopened app can show and re-apply them.
pub fn saved() -> Option<(f32, f32)> {
    let bytes = std::fs::read(data_dir().ok()?.join("color-state.json")).ok()?;
    let (saturation, shadow) = serde_json::from_slice(&bytes).ok()?;
    validate(saturation, shadow).ok()?;
    Some((saturation, shadow))
}
/// Best effort: a failed write only means the sliders start at their defaults next launch.
fn remember(values: Option<(f32, f32)>) {
    let Ok(path) = data_dir().map(|d| d.join("color-state.json")) else {
        return;
    };
    let _ = match values.and_then(|v| serde_json::to_vec(&v).ok()) {
        Some(data) => std::fs::write(path, data),
        None => std::fs::remove_file(path),
    };
}
fn stored_ramp(gamma: &[Vec<u16>]) -> Result<Ramp, String> {
    if gamma.len() != 3 || gamma.iter().any(|c| c.len() != 256) {
        return Err("Invalid saved gamma ramp".into());
    }
    Ok(std::array::from_fn(|c| {
        std::array::from_fn(|i| gamma[c][i])
    }))
}
fn save_baseline(path: &std::path::Path, b: &Baseline) -> Result<(), String> {
    use std::io::Write;
    let data = serde_json::to_vec(b).map_err(|e| e.to_string())?;
    let temp = path.with_extension("tmp");
    let mut file = std::fs::File::create(&temp).map_err(|e| e.to_string())?;
    file.write_all(&data)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    drop(file);
    std::fs::rename(&temp, path).map_err(|e| e.to_string())
}
fn load_baseline(display: u32, monitor: &str) -> Result<Baseline, String> {
    let bytes = std::fs::read(baseline_path(display)?)
        .map_err(|e| format!("Saved color baseline unavailable: {e}"))?;
    let b: Baseline =
        serde_json::from_slice(&bytes).map_err(|e| format!("Invalid saved colors: {e}"))?;
    if b.display != display
        || b.monitor != monitor
        || b.settings.version != VERSION
        || !b
            .settings
            .matrix1
            .iter()
            .chain(b.settings.matrix2.iter())
            .all(|v| v.is_finite())
    {
        return Err("Saved colors do not match this primary monitor".into());
    }
    if let Some(d) = &b.dvc {
        validate_dvc(d)?;
    }
    if let Some(g) = &b.gamma {
        stored_ramp(g)?;
    }
    Ok(b)
}
fn capture_baseline(display: u32, monitor: &str, current: &Snapshot) -> Result<Baseline, String> {
    let path = baseline_path(display)?;
    let mut b = if path.exists() {
        load_baseline(display, monitor)?
    } else {
        Baseline {
            display,
            monitor: monitor.into(),
            settings: current.settings.clone(),
            dvc: None,
            gamma: None,
        }
    };
    if b.dvc.is_none() || b.gamma.is_none() {
        // Migrate 0.2 baselines without discarding their saved original CSC.
        b.dvc.get_or_insert_with(|| current.dvc.clone());
        b.gamma
            .get_or_insert_with(|| current.gamma.iter().map(|r| r.to_vec()).collect());
        save_baseline(&path, &b)?;
    }
    Ok(b)
}
fn validate(saturation: f32, shadow: f32) -> Result<(), String> {
    if !saturation.is_finite()
        || !(1.0..=2.75).contains(&saturation)
        || !shadow.is_finite()
        || !(1.0..=1.8).contains(&shadow)
    {
        Err("Vibrance must be 100-275% and shadow boost 1.00-1.80".into())
    } else {
        Ok(())
    }
}
pub fn apply(saturation: f32, shadow: f32) -> Result<(), String> {
    validate(saturation, shadow)?;
    let _guard = LOCK.lock().map_err(|e| e.to_string())?;
    let d = Driver::open()?;
    let display = d.primary()?;
    let (name, monitor) = primary_monitor()?;
    crate::display::ensure_sdr(&name)?;
    let handle = d.display_handle(&name)?;
    let gamma = GammaDc::open(&name)?;
    let current = Snapshot {
        settings: d.read(display)?,
        dvc: d.read_dvc(handle)?,
        gamma: gamma.read()?,
    };
    let b = capture_baseline(display, &monitor, &current)?;
    let original = stored_ramp(b.gamma.as_ref().ok_or("Saved gamma missing")?)?;
    let target = desired_csc(saturation);
    let lift = knee_ramp(
        &pop_ramp(&shadow_ramp(&original, shadow), saturation),
        headroom(saturation),
    );
    let mut output = LiveOutput {
        driver: &d,
        display,
        handle,
        gamma: &gamma,
    };
    apply_transaction(&mut output, &current, &target, &lift)?;
    remember(Some((saturation, shadow)));
    Ok(())
}
pub fn reset() -> Result<(), String> {
    let _guard = LOCK.lock().map_err(|e| e.to_string())?;
    let d = Driver::open()?;
    let display = d.primary()?;
    let (name, monitor) = primary_monitor()?;
    crate::display::ensure_sdr(&name)?;
    let handle = d.display_handle(&name)?;
    let gamma = GammaDc::open(&name)?;
    let b = load_baseline(display, &monitor)?;
    let current = Snapshot {
        settings: d.read(display)?,
        dvc: d.read_dvc(handle)?,
        gamma: gamma.read()?,
    };
    let target = Snapshot {
        settings: b.settings,
        dvc: b.dvc.unwrap_or_else(|| current.dvc.clone()),
        gamma: match b.gamma {
            Some(g) => stored_ramp(&g)?,
            None => current.gamma,
        },
    };
    let mut output = LiveOutput {
        driver: &d,
        display,
        handle,
        gamma: &gamma,
    };
    if let Err(e) = rollback(&mut output, &target) {
        return match rollback(&mut output, &current) {
            Ok(()) => Err(e),
            Err(r) => Err(format!("{e}; recovery also failed: {r}")),
        };
    }
    remember(None);
    std::fs::remove_file(baseline_path(display)?).map_err(|e| e.to_string())
}

/// Explicit recovery for the old app's unsaved gamma changes. Confirmation is in UI.
pub fn clear_legacy() -> Result<(), String> {
    let _guard = LOCK.lock().map_err(|e| e.to_string())?;
    let d = Driver::open()?;
    let display = d.primary()?;
    let (name, monitor) = primary_monitor()?;
    crate::display::ensure_sdr(&name)?;
    let handle = d.display_handle(&name)?;
    let gamma = GammaDc::open(&name)?;
    let current = Snapshot {
        settings: d.read(display)?,
        dvc: d.read_dvc(handle)?,
        gamma: gamma.read()?,
    };
    capture_baseline(display, &monitor, &current)?;
    let mut output = LiveOutput {
        driver: &d,
        display,
        handle,
        gamma: &gamma,
    };
    apply_transaction(&mut output, &current, &neutral_csc(), &linear_ramp())?;
    remember(None);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identity_neutrals_and_luminance_are_preserved() {
        assert_eq!(std::mem::size_of::<Csc>(), 124);
        for s in [1., 1.06, 1.12, 1.5, 2., 3.] {
            let m = matrix(s);
            for row in 0..3 {
                assert!((m[row * 4..row * 4 + 3].iter().sum::<f32>() - 1.).abs() < 1e-6);
            }
            let rgb = [0.3, 0.5, 0.4];
            let w = [0.2126, 0.7152, 0.0722];
            let out: [f32; 3] =
                std::array::from_fn(|i| (0..3).map(|j| m[i * 4 + j] * rgb[j]).sum());
            let before: f32 = rgb.iter().zip(w).map(|(x, w)| x * w).sum();
            let after: f32 = out.iter().zip(w).map(|(x, w)| x * w).sum();
            assert!((before - after).abs() < 1e-6);
        }
        assert_eq!(matrix(1.), [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0.]);
    }
    #[test]
    fn extended_saturation_is_not_lossless_at_gamut_edges() {
        // Pure red exceeds the display range even at a gentle boost: never claim lossless.
        let m = matrix(1.06);
        assert!(m[0] > 1. && m[4] < 0. && m[8] < 0.);
    }
    #[test]
    fn invalid_values_are_rejected_before_loading_driver() {
        for s in [1., 1.6, 2.75] {
            assert!(validate(s, 1.0).is_ok());
        }
        for s in [f32::NAN, f32::INFINITY, 0., 2.76, 3.] {
            assert!(apply(s, 1.0).is_err());
        }
        for g in [f32::NAN, f32::NEG_INFINITY, 0.99, 1.81] {
            assert!(apply(1.2, g).is_err());
        }
    }

    #[test]
    fn shadow_lift_preserves_endpoints_order_and_saved_calibration() {
        let base = linear_ramp();
        assert_eq!(shadow_ramp(&base, 1.0), base);
        for g in [1.08, 1.25, 1.8] {
            let ramp = shadow_ramp(&base, g);
            for channel in ramp {
                assert_eq!((channel[0], channel[255]), (0, 65535));
                assert!(channel.windows(2).all(|w| w[1] > w[0]));
                assert!(channel[40] > base[0][40]);
                // Old curve lifted near-black by 50%+ and washed terrain out.
                assert!(channel[26] < base[0][26] * 2);
                assert!(channel.windows(2).all(|w| w[1] - w[0] >= 180));
            }
        }
        let calibrated = std::array::from_fn(|c| {
            std::array::from_fn(|i| {
                ((i as f32 / 255.).powf(1.05 + c as f32 * 0.03) * 65535.).round() as u16
            })
        });
        assert_eq!(shadow_ramp(&calibrated, 1.0), calibrated);
        assert_ne!(shadow_ramp(&calibrated, 1.25), shadow_ramp(&base, 1.25));
        assert!(!ramps_close(&base, &shadow_ramp(&base, 1.8)));
    }

    #[test]
    fn green_is_protected_above_160_and_neutrals_stay_exact() {
        assert_eq!(desired_csc(1.6).matrix1, scaled(1.6));
        let m = desired_csc(2.75).matrix1.map(|v| v * headroom(2.75));
        for row in 0..3 {
            assert!((m[row * 4..row * 4 + 3].iter().sum::<f32>() - 1.).abs() < 1e-5);
            assert_eq!(m[row * 4 + 3], 0.);
        }
        // Sunlit grass: plain 275% pushes green to 1.13 (clipped flat); protected stays near range.
        let grass = [0.5, 0.9, 0.25];
        let green = |m: [f32; 12]| (0..3).map(|j| m[4 + j] * grass[j]).sum::<f32>();
        assert!(green(matrix(2.75)) > 1.12 && green(m) < 1.04);
        assert_eq!(desired_csc(2.75).matrix1[..4], scaled(2.75)[..4]); // red still gets the full boost
    }
    fn scaled(s: f32) -> [f32; 12] {
        matrix(s).map(|v| v / headroom(s))
    }

    #[test]
    fn pop_curve_adds_midtone_contrast_without_flattening_ends() {
        let base = linear_ramp();
        assert_eq!(pop_ramp(&base, 1.), base);
        for s in [1.35, 1.75, 2.75] {
            let r = pop_ramp(&base, s)[0];
            assert_eq!((r[0], r[255]), (0, 65535));
            assert!(r.windows(2).all(|w| w[1] - w[0] >= 190), "{s}"); // identity step is 257
            assert!(r[128] - r[127] > 257 && r[64] < base[0][64] && r[192] > base[0][192]);
        }
    }

    #[test]
    fn headroom_keeps_boosted_red_detail_and_restores_midtones() {
        let base = linear_ramp();
        assert_eq!(knee_ramp(&base, headroom(1.)), base);
        for s in [1.2, 1.6, 2.75] {
            let h = headroom(s);
            let ramp = knee_ramp(&base, h)[0];
            assert_eq!((ramp[0], ramp[255]), (0, 65535));
            assert!(ramp.windows(2).all(|w| w[1] >= w[0]));
            // Mid grey 0.5 arrives as 0.5/h and must come back out as 0.5.
            let mid = sample(&ramp, 0.5 / h) as f32 / 65535.;
            assert!((mid - 0.5).abs() < 0.005, "{s}");
        }
        // Red surface (0.8,0.15,0.1)±0.1 at 160%: red output 0.95..1.27 used to clip to one value.
        let (s, h) = (1.6, headroom(1.6));
        let ramp = knee_ramp(&base, h)[0];
        let red = |r: f32| {
            let out: f32 = matrix(s)[..3].iter().zip([r, 0.15, 0.1]).map(|(m, c)| m * c).sum();
            sample(&ramp, out / h)
        };
        assert!(red(0.8) > red(0.7) + 1000 && red(0.9) > red(0.8) + 300);
    }

    struct FakeOutput {
        state: Snapshot,
        events: Vec<String>,
        fail_once: Option<&'static str>,
    }
    impl FakeOutput {
        fn record(&mut self, stage: &'static str, event: String) -> Result<(), String> {
            self.events.push(event);
            if self.fail_once == Some(stage) {
                self.fail_once = None;
                Err(format!("failed {stage}"))
            } else {
                Ok(())
            }
        }
    }
    impl Output for FakeOutput {
        fn csc(&mut self, value: &Csc) -> Result<(), String> {
            self.state.settings = value.clone();
            self.record("csc", "csc".into())
        }
        fn dvc(&mut self, level: i32) -> Result<(), String> {
            self.state.dvc.current = level;
            self.record("native", format!("native:{level}"))
        }
        fn gamma(&mut self, value: &Ramp) -> Result<(), String> {
            if self.state.gamma == *value {
                return Ok(());
            }
            self.state.gamma = *value;
            self.record("gamma", "gamma".into())
        }
    }
    fn existing_filter() -> Snapshot {
        Snapshot {
            settings: desired_csc(1.3),
            dvc: Dvc {
                version: 0x10014,
                current: 100,
                minimum: 0,
                maximum: 100,
                neutral: 50,
            },
            gamma: linear_ramp(),
        }
    }

    #[test]
    fn neutralizes_native_vibrance_before_replacing_active_filter() {
        let previous = existing_filter();
        let mut output = FakeOutput {
            state: previous.clone(),
            events: vec![],
            fail_once: None,
        };
        let gamma = shadow_ramp(&previous.gamma, 1.25);
        apply_transaction(&mut output, &previous, &desired_csc(1.5), &gamma).unwrap();
        assert_eq!(output.events, vec!["native:50", "csc", "gamma"]);
        assert_eq!(output.state.dvc.current, 50); // zero would make it grayscale
        assert_eq!(output.state.settings.matrix1, scaled(1.5));
        assert_eq!(output.state.gamma, gamma);
    }

    #[test]
    fn every_partial_apply_failure_restores_all_previous_settings() {
        for stage in ["native", "csc", "gamma"] {
            let previous = existing_filter();
            let mut output = FakeOutput {
                state: previous.clone(),
                events: vec![],
                fail_once: Some(stage),
            };
            let gamma = shadow_ramp(&previous.gamma, 1.25);
            assert!(apply_transaction(&mut output, &previous, &desired_csc(1.5), &gamma).is_err());
            assert_eq!(output.state.settings, previous.settings, "{stage}");
            assert_eq!(output.state.dvc, previous.dvc, "{stage}");
            assert_eq!(output.state.gamma, previous.gamma, "{stage}");
        }
    }

    #[test]
    fn gamma_is_not_rewritten_when_shadow_boost_is_unchanged() {
        let previous = existing_filter();
        let mut output = FakeOutput {
            state: previous.clone(),
            events: vec![],
            fail_once: None,
        };
        apply_transaction(&mut output, &previous, &desired_csc(1.), &previous.gamma).unwrap();
        assert_eq!(output.events, vec!["native:50", "csc"]);
        assert_eq!(output.state.settings.use_matrix1, 0);
    }

    #[test]
    fn version_02_baselines_remain_readable_without_new_fields() {
        let old = serde_json::json!({"display":1,"monitor":"example","settings":desired_csc(1.3)});
        let baseline: Baseline = serde_json::from_value(old).unwrap();
        assert!(baseline.dvc.is_none() && baseline.gamma.is_none());
        assert_eq!(baseline.settings.matrix1, scaled(1.3));
        assert!(stored_ramp(&vec![vec![0; 255]; 3]).is_err());
    }
}
