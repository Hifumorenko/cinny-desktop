fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new().app_manifest(
            tauri_build::AppManifest::new()
                .commands(&[
                    "show_desktop_notification",
                    "set_push_to_talk_binds",
                    "get_quit_on_close",
                    "set_quit_on_close",
                ]),
        ),
    )
    .expect("failed to build the desktop command permissions");
}
