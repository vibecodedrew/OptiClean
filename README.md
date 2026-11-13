# OptiClean

A Windows 11 utility for inspecting and applying individual system settings, cleaning selected caches, managing startup apps, and exploring hardware-aware BIOS/GPU guidance. Rust/Tauri 2 backend; F#/Fable/Elmish/Feliz frontend. MIT licensed.

## Known Issues ##
Issue 1: When the app starts up it does take a minute for all tweaks to fully show up inside of OptiClean, kind of like a startup lag if you wanna call it that.

Likely cause (from reading the code, not measured):
 When the app opens, the frontend calls list_tweaks once and shows nothing until it returns. In src-tauri/src/main.rs, that command loops over every tweak and calls reg::is_on for each one, one after another.
 Most tweaks are plain registry reads and are instant. But about seven of them (for example Ultimate Performance, USB suspend, PCIe power saving, hibernation, windowed-game optimizations, VRR and Nagle) are Action::Script tweaks. For those, is_on calls reg::ps, which starts a brand-new hidden powershell.exe just to run one small check. Each PowerShell start costs a noticeable fraction of a second or more, and they run back to back.
 On top of that, startup also launches two more PowerShell processes at the same time: hardware (several CIM queries) and recovery_status (reads restore points).

 Ideas for a fix:
 - Return the registry-based tweaks immediately and load the script-based ones afterwards. The frontend already supports this: tweak_status exists per tweak, and a missing status is shown as "still reading".
 - Run all the script checks in a single PowerShell process instead of one process each.
 - Run the checks in parallel instead of in sequence.
 - Replace the PowerShell checks with native calls where possible (for example reading the active power plan through the Windows power API instead of powercfg).

## What changed in 0.2.4

- Remove the "Turn off Widgets" switch. Windows' UCPD driver denies that policy key to every program, administrators included, so it could only fail with "Access is denied". Widgets stays removable on the Remove apps page.
- Fix "Visual effects: best performance" reading as not applied whenever "Remove the menu delay" was changed: both wrote `MenuShowDelay`. The menu delay now belongs to its own switch only, and visual effects keeps font smoothing on.
- Explain in the Ultimate Performance details that vendor utilities which apply a power profile at startup switch the plan back to Balanced.
- After a change, the status line says when a sign-out, reboot or Explorer restart is still needed.
- Refresh the interface: pill tabs with a count of applied tweaks per category, a labelled hardware panel, a progress meter in the header, and tweak cards that show clearly when they are on. Same layout and pure-black background; motion stays brief and only answers an action.
- Harden the elevated process: PowerShell is started by full path with `PATH` and `PSModulePath` pinned to Windows' own folders; cleaner folders come from Windows APIs instead of environment variables, and per-user folders must resolve inside the user profile; startup switches only accept entries that exist.
- Source release: drop icon files the build does not use.

## What changed in 0.2.3

- Detects CPU, GPUs, board, BIOS and memory on demand. Attempts PSU identification; ordinary ATX PSUs do not expose a reliable model/wattage interface, so missing details must be entered manually.
- BIOS/GPU guidance uses detected or entered hardware. PSU, cooler, measured temperatures and explicit compatibility confirmations gate conservative manual tuning suggestions. Unknown and unsupported systems stay at stock.
- Supports extended NVIDIA saturation at 100-275%, neutralizes native Digital Vibrance before the app matrix, and restores shadow boost. Presets: Original, Competitive, Vivid and Xtra.
- Applies slider edits automatically, combining rapid changes into live updates with a single driver call active. No Apply button, confirmation checkbox or idle polling. Windows display mode is checked automatically before changes.
- Reuses one verified OptiClean System Restore baseline across app launches. The Recovery page opens the Windows wizard for review; no unattended restore or reboot. OptiClean will never restore anything for you, but it will create a point of restoration.
- Adds documented indexing-backoff, Edge sleeping-tab and network-thumbnail-cache policies. Expected gains are conditional, never invented FPS promises.
- Keeps the original tabs and toggle layout. Removes the ON DEMAND badge and Color banner. Reveals a black native window after the first UI commit, without a full-page fade-in or startup color changes. Release history lives in [CHANGELOG.md](CHANGELOG.md).

## Requirements and build

Windows 11 x64, Microsoft Edge WebView2 Runtime, .NET SDK 10, Node.js 22.12+ (or a current compatible LTS), Rust stable, and Visual Studio C++ Build Tools with the Windows SDK. Use a Developer PowerShell if the linker is not on PATH.

```powershell
npm.cmd ci
dotnet tool restore --tool-manifest dotnet-tools.json
dotnet restore src/App.fsproj
npm.cmd run build
npm.cmd test
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
npm.cmd run tauri -- build --no-bundle
```

The executable is `src-tauri/target/release/opticlean.exe`. Release builds request administrator rights to manage machine settings. The source release does not include binaries, PDBs or machine caches. For development, run `npm.cmd run tauri -- dev`; machine settings still require elevation. In PowerShell, `npm.cmd` avoids execution-policy restrictions on the npm PowerShell shim.

After building the frontend, the equivalent direct production build is `cargo build --release --manifest-path src-tauri/Cargo.toml`. The `custom-protocol` feature is on by default and embeds the frontend; `tauri dev` turns it off to use the Vite server.

To preview the UI without accessing Windows settings, run `npm.cmd run dev`, then open `http://localhost:5173/tests/preview.html`. This synthetic fixture uses example hardware and intercepts native commands; it is not included in the production bundle. `npm test` expects the Fable output produced by `npm run build`.

## Color behavior and limitations

Extended saturation uses the existing NVIDIA output-matrix interface, beyond the control-panel slider's usual range. It supports the primary NVIDIA-connected SDR display only. The interface is undocumented and driver-dependent; AMD/Intel, hybrid laptop outputs and HDR need vendor controls instead. No shaders, overlays, game hooks, frame processing, display polling or background service are used. Close the application after applying to release its UI processes and memory.

Apply first sets NVIDIA Digital Vibrance to the driver-reported neutral value (usually 50%, not 0%), then applies the app matrix. Shadow boost composes a smooth curve with the saved gamma ramp and fades toward white. At 1.00 it uses the saved ramp. Contrast, brightness, resolution and texture filtering are not changed. Original means neutral native vibrance, no added matrix and the saved tone curve. Existing output filters are replaced while these settings are active; Restore previous settings restores their saved originals. No boost is reapplied on startup.

Saturation above 100% can exceed the display gamut and clip colors, including texture detail in Fortnite/COD. Neutral grays and the matrix's weighted pre-clamp luminance are preserved, but this is not a nonlinear, pixel-aware gamut mapper. Start with a small manual boost. Xtra selects 200% saturation with no extra shadow lift; it cannot preserve image quality 1:1. Actual FPS/input-delay impact has not been benchmarked. Display adjustments may not be captured in clips recorded before the output stage.

Before display changes, native vibrance, matrix, monitor identity and gamma are saved under `%LOCALAPPDATA%/OptiClean`. Readback is checked, and partial failures attempt to restore every previous stage. Existing 0.2 baselines gain the native/gamma values observed at upgrade while retaining the original saved matrix.

The older app never saved its gamma baseline. The explicit Clear legacy filter action resets gamma and output transforms, including custom calibration, and asks for confirmation. Reapply calibration afterward. Normal controls do not perform that legacy reset. Presets apply immediately and sliders update automatically without a checkbox. Windows HDR/advanced-color mode is checked directly before output changes; unsupported modes report an error without changing settings. Rapid edits are combined at 75 ms intervals; calls never overlap, and slow calls retain only the newest queued values. Recovery cancels pending edits and waits for any active call. There is no idle update loop.

## Recovery and tuning

Before Windows-setting, startup, app-removal or cleanup operations, the backend verifies or creates the baseline. A machine-wide named mutex prevents simultaneous creation; legacy points named `OptiClean` are reused. Windows can expire points, so a missing baseline must be recreated before further operations. Windows' normal checkpoint frequency limit is respected. Other applications may create their own restore points; OptiClean does not delete them or force a frequency override.

System Restore is not a backup of personal files and cannot recover cleaned files or undo BIOS/GPU overclocks. Per-tweak Undo uses documented default values, not an exact snapshot of previous custom values. App removal affects the current account only. Existing advanced tweaks remain opt-in; some have security, compatibility or power trade-offs explained in the UI. Nothing is automatically applied on launch.

No OC is guaranteed safe. The guide never sets clocks, voltage, firmware or power limits. Follow the exact hardware vendor's limits and warranty terms. Model names and PSU watts alone do not establish compatibility or electrical headroom. The small suggested offsets are experimental starting points requiring stock stability, cooling checks, stress tests, game tests and rollback after any errors.

## Privacy and release contents

No telemetry, account, API key or network service is required at runtime. Hardware forms stay in memory. Only local original color settings/monitor identity are persisted for recovery. Browser context menus and print/save shortcuts are disabled. Source preparation uses an explicit allowlist; private machine notes, histories, generated schemas, build outputs, dependency caches, logs, profiles and credentials are excluded.

See [VALIDATION.md](VALIDATION.md) for exact verified coverage and remaining gaps, and [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for attribution.

## Technical references

- [Microsoft: optimizations for windowed games](https://support.microsoft.com/en-us/windows/hardware/display-graphics/optimizations-for-windowed-games-in-windows-11)
- [Microsoft: Search DisableBackoff policy](https://learn.microsoft.com/en-us/windows/client-management/mdm/policy-csp-search#disablebackoff)
- [Microsoft: Edge sleeping tabs policy](https://learn.microsoft.com/en-us/deployedge/microsoft-edge-browser-policies/sleepingtabsenabled)
- [Microsoft: network thumbnail cache policy](https://learn.microsoft.com/en-us/windows/client-management/mdm/policy-csp-admx-thumbnails#disablethumbsdbonnetworkfolders)
- [Microsoft: checkpoint limits](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.management/checkpoint-computer?view=powershell-5.1)
- [NVIDIA: output color processing](https://www.nvidia.com/content/Control-Panel-Help/vLatest/en-us/mergedProjects/nvdsp/CS_Adjust_Color_Settings_Advanced.htm)
- [AMD: Ryzen Master tuning and warranty terms](https://www.amd.com/en/products/software/ryzen-master.html)
