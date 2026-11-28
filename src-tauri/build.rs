fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new().app_manifest(
            tauri_build::AppManifest::new()
                .commands(&["show_desktop_notification", "set_push_to_talk_binds"]),
        ),
    )
    .expect("failed to build the desktop command permissions");
}
