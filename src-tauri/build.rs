fn main() {
    let mut attrs = tauri_build::Attributes::new();
    // Only release builds demand admin, so `tauri dev` and `cargo test` run from a normal terminal.
    if std::env::var("PROFILE").as_deref() == Ok("release") {
        attrs = attrs.windows_attributes(
            tauri_build::WindowsAttributes::new().app_manifest(include_str!("admin.manifest")),
        );
    }
    tauri_build::try_build(attrs).expect("tauri build failed");
}
