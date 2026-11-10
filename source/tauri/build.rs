fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new().windows_attributes(
            tauri_build::WindowsAttributes::new_without_app_manifest()
                .app_manifest(include_str!("windows-app-manifest.xml")),
        ),
    )
    .expect("failed to run tauri build");
}
