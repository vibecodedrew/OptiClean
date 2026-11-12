# OptiClean

Windows 11 desktop utility: Rust/Tauri 2 backend, F#/Fable/Elmish/Feliz frontend.

- Build frontend with `npm run build`; verify Rust with `cargo test --manifest-path src-tauri/Cargo.toml` and `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`.
- Do not run ignored tests: they modify the host's drivers or Windows settings.
- Hardware guidance must use detected or user-supplied specifications. Unknown hardware stays unknown; never assume the developer's PC.
- Never apply BIOS, voltage, clock or power-limit changes automatically. Overclocking guidance must explain validation and rollback and cannot promise stability.
- Keep color adjustments out of the game's render path. Preserve gamma/calibration, save original driver values, and report unsupported hardware honestly.
- Reuse one verified OptiClean restore point across restarts. Protect creation against concurrent requests. Recovery must open the Windows wizard for user review.
- Keep background work on demand, animations brief, and honor reduced motion. Do not add telemetry, background polling, printing or context menus.
- Keep usage of the program low, find conventional ways to ensure the process is smooth, fast, and optimized to ensure we are not losing performance.
- Keep usage low with on-demand work and no continuous game/display monitoring. Preserve the existing app structure while improving hardware detection, safe tweaks, UI polish, and extended vibrance.
- Use Higgsfield for UI inspiration or assets if needed. Make the UI striking while keeping CPU, GPU, memory and idle usage low. Prefer brief interaction animations; avoid continuous effects, video backgrounds, polling and performance-heavy assets.
- Keep public source in `OpenSourced` free of build caches, private hardware notes, API keys, anything sensitive that could be used against the creator, personal paths, and machine logs. Update its changelog and build instructions when preparing a release.
- Make sure you keep the app as it was really, just adding in stuff, improving UI, detection for hardware, improving color boosting to ensure image quality is kept as intended, but boosting colors majorly for video/clips/eyes to look way better.
