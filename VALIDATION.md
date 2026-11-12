# Validation

## Verified locally (0.2.4)

- F# type-check and Fable JavaScript compilation.
- Vite production bundle generation.
- Sixteen JavaScript tests: seven live-color queue tests, two startup-reveal tests and seven hardware-guidance tests. Queue coverage includes rapid changes, slow drivers, no overlapping writes, presets, cancellation/recovery ordering, error reporting and zero idle work.
- Twenty-four Rust tests, including native-neutral-before-matrix ordering, replacing active legacy filters, rollback after each stage fails, shadow-ramp endpoints/order/calibration, gamma readback comparison, baseline migration, and rejecting HDR/advanced/unknown color modes while allowing known SDR. Output writes are mocked.
- Mocked PowerShell recovery checks for baseline reuse, absent-checkpoint failure reporting and mutex release. The mocks never invoke the real Windows restore cmdlets.
- Cleaner fixture and tweak-catalog consistency checks.
- New in 0.2.4: no two tweaks write the same registry value; no tweak writes a key Windows protects; cleaner folders and PowerShell ignore poisoned environment variables; only listed startup entries can be switched.
- Rust Clippy with warnings denied and an optimized Windows release build.
- Public source allowlist and local scan for private paths, credentials, logs and generated/build files.
- A read-only Windows DisplayConfig query successfully identified the current primary display as SDR without user confirmation or display-setting changes.

## Not established by these checks

- The 0.2.4 tweak fixes were checked against the registry, live settings and the Windows event log, not through a sign-out or reboot cycle.
- A browser/UI automation surface was not connected, so visual inspection was unavailable. `tests/preview.html` supplies a synthetic UI fixture for manual review.
- Read-only live NVIDIA queries succeeded for initialization, primary-display CSC, display association and native-vibrance range. Native vibrance was at maximum and the saved baseline had an active matrix. No game/driver/display mutation tests were run. Actual appearance, recovery, HDR handling, frame times, input delay and idle CPU/GPU/memory use still require target-PC measurements.
- Real Windows System Restore creation/restoration and elevated Windows-setting operations were not exercised. Two host-registry/tweak integration tests remain ignored by default because they change host settings.
- OC suggestions are not a hardware validation or a stability guarantee.

The environment restricted parent-directory enumeration by Fable, so local compilation used a temporary drive alias and the already-installed Fable tool with `--noRestore`. A standard unrestricted Windows checkout uses the README build commands. No personal tool paths or alias configuration are required by the public project.
