# Changelog

## 0.2.5 - 2026-11-13

- Custom title bar with minimize and close; the Windows frame is gone. Fixed 1380x880 window, centered, no resize or maximize.
- Status messages clear when switching tabs instead of following you to the next page.
- Flatter interface with no blurred glows and underline tabs, so repaints cost less.
- Recovery page is a single row: baseline status, Open System Restore, and a Create baseline button only while Windows has no OptiClean baseline.
- Color studio reads the saved saturation and shadow values on startup, so sliders match what the driver is running after a relaunch.
- Fix a plain `cargo build --release` producing a binary that loads the Vite dev URL and shows no window. The `custom-protocol` feature is now a default feature, so release builds always embed the frontend.

## 0.2.4 - 2026-11-12

- Remove the "Turn off Widgets" switch. Windows' UCPD driver denies that policy key to every program, administrators included, so it could only fail with "Access is denied". Widgets stays removable on the Remove apps page.
- Fix "Visual effects: best performance" reading as not applied whenever "Remove the menu delay" was changed: both wrote `MenuShowDelay`. The menu delay now belongs to its own switch only, and visual effects keeps font smoothing on.
- Explain in the Ultimate Performance details that vendor utilities which apply a power profile at startup switch the plan back to Balanced.
- After a change, the status line says when a sign-out, reboot or Explorer restart is still needed.
- Refresh the interface: pill tabs with a count of applied tweaks per category, a labelled hardware panel, a progress meter in the header, and tweak cards that show clearly when they are on. Same layout and pure-black background; motion stays brief and only answers an action.
- Harden the elevated process: PowerShell is started by full path with `PATH` and `PSModulePath` pinned to Windows' own folders; cleaner folders come from Windows APIs instead of environment variables, and per-user folders must resolve inside the user profile; startup switches only accept entries that exist.
- Source release: drop icon files the build does not use.

## 0.2.3 - 2026-11-10

- Raise extended vibrance from 200% to 300% in the slider and backend validation. Keep the display-wide NVIDIA output path and existing presets.
- Remove the SDR confirmation checkbox and its gate; sliders and presets apply without any checkbox.
- Check the actual Windows color mode on the primary display before output changes, including mirrored targets. Use the newer HDR/SDR query when available and the legacy query on older Windows builds.
- Keep checks read-only and tied to user changes, without PowerShell, polling or startup color writes. Unsupported modes report an error before settings are modified.

## 0.2.2 - 2026-11-10

- Remove the Color Apply button; both sliders apply automatically after SDR confirmation.
- Combine rapid edits into live updates at 75 ms intervals, with at most one native call active and one latest value waiting. Keep sliders responsive during slow driver calls.
- Prevent old results from moving slider values backward. Presets apply immediately; recovery cancels queued edits and waits for active calls before restoring settings.
- Add seven queue tests for idle behavior, rapid edits, slow drivers, presets, cancellation, recovery ordering and errors.

## 0.2.1 - 2026-11-10

- Remove the active-baseline guard that could block vibrance over the older filter.
- Neutralize NVIDIA Digital Vibrance before applying the app matrix; preserve its previous value for recovery.
- Restore shadow boost with a smooth ramp composed against saved calibration. Verify readback because Windows can silently ignore gamma writes.
- Migrate native vibrance and gamma into saved baselines, and roll back all stages after partial failures.
- Simplify Color to two sliders and Original, Competitive, Vivid and Xtra presets. Remove its blue banner and the ON DEMAND badge. Xtra is maximum saturation and is not claimed to be lossless.
- Reveal the black native window after React mounts, avoiding hidden-window animation-frame waits, full-page fade-in and startup color writes.
- Add ordering, rollback, shadow-ramp, baseline migration and startup-reveal tests. Keep all display work event-driven, without polling.

## 0.2.0 - 2026-11-10

- Replace personal hardware assumptions with on-demand CIM detection and an editable hardware form, including PSU/cooling/temperature requirements.
- Generate conservative, manual BIOS/GPU guidance with support, stability and thermal gates. Keep unknown, mobile and unsupported configurations on stock guidance.
- Retain the 100-200% extended color range; add Subtle/Vivid presets and explicit Apply. Remove startup auto-apply and normal gamma/contrast modification. Save and validate original output settings before changes, with rollback on apply failure and monitor-matched reset.
- Provide an explicit, confirmed recovery path for the older filter's unsaved gamma/contrast changes.
- Verify and reuse a single Windows restore baseline, serialize creation with a named mutex, preserve Windows' normal frequency policy, and add a Recovery page.
- Add indexing backoff, Edge sleeping tabs and network-thumbnail caching policies with conditional benefit descriptions.
- Refresh the existing black/blue UI with cards and brief transitions, honoring reduced motion. Remove the bottom activity log, Windows right-click tweaks, native browser context menus and print/save shortcuts.
- Read tweak status sequentially in a background backend command instead of starting many PowerShell processes concurrently.
- Restrict app removal to the current account, restrict NVIDIA DLL loading to System32, and add a production content-security policy.
- Add OC-policy, color-math and mocked-recovery checks. Make host-registry integration testing explicitly opt-in.
- Prepare an allowlisted public source release with license, attribution, reproducible build instructions and validation limits.
