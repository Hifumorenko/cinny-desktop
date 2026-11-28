#[derive(Clone, serde::Deserialize, serde::Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum NotificationTarget {
    Test,
    Invites,
    Room {
        #[serde(rename = "roomId")]
        room_id: String,
        #[serde(rename = "eventId")]
        event_id: String,
    },
}

#[derive(Default)]
pub struct NotificationState {
    #[cfg(target_os = "windows")]
    registration_lock: std::sync::Mutex<()>,
    #[cfg(target_os = "windows")]
    recent_avatars:
        std::sync::Mutex<std::collections::HashMap<std::path::PathBuf, std::time::Instant>>,
}

#[cfg(target_os = "windows")]
fn register_notification_identity(app: &tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;

    let state = app.state::<NotificationState>();
    let _registration = state.registration_lock.lock().map_err(|e| e.to_string())?;

    // Register only Cinny's per-user notification identity. This also works
    // for a portable EXE and does not require an installer or administrator.
    let icon_dir = app.path().app_cache_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&icon_dir).map_err(|e| e.to_string())?;
    let icon_path = icon_dir.join("notification-icon.png");
    let icon_bytes = include_bytes!("../icons/128x128.png");
    if std::fs::read(&icon_path).ok().as_deref() != Some(icon_bytes.as_slice()) {
        std::fs::write(&icon_path, icon_bytes).map_err(|e| e.to_string())?;
    }

    let key = windows_registry::CURRENT_USER
        .create(format!(
            r"Software\Classes\AppUserModelId\{}",
            app.config().identifier
        ))
        .map_err(|e| e.to_string())?;
    key.set_string(
        "DisplayName",
        app.config().product_name.as_deref().unwrap_or("Cinny"),
    )
    .map_err(|e| e.to_string())?;
    key.set_string("IconUri", icon_path.to_string_lossy().as_ref())
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn cache_avatar(app: &tauri::AppHandle, bytes: &[u8]) -> Option<std::path::PathBuf> {
    use sha2::{Digest, Sha256};
    use tauri::Manager;

    if bytes.len() > 256 * 1024 {
        return None;
    }
    let mut reader =
        image::ImageReader::with_format(std::io::Cursor::new(bytes), image::ImageFormat::Png);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(128);
    limits.max_image_height = Some(128);
    limits.max_alloc = Some(1024 * 1024);
    reader.limits(limits);
    reader.decode().ok()?;

    let state = app.state::<NotificationState>();
    let _registration = state.registration_lock.lock().ok()?;
    let directory = app
        .path()
        .app_cache_dir()
        .ok()?
        .join("notification-avatars");
    std::fs::create_dir_all(&directory).ok()?;
    let path = directory.join(format!("{:x}.png", Sha256::digest(bytes)));
    if std::fs::read(&path).ok().as_deref() != Some(bytes) {
        std::fs::write(&path, bytes).ok()?;
    }
    let mut recent = state.recent_avatars.lock().ok()?;
    recent.retain(|_, used| used.elapsed().as_secs() <= 60);
    recent.insert(path.clone(), std::time::Instant::now());
    // Keep recent toast images intact while bounding the longer-lived cache.
    let entries = match std::fs::read_dir(&directory) {
        Ok(entries) => entries,
        Err(_) => return Some(path),
    };
    let mut cached: Vec<_> = entries
        .filter_map(Result::ok)
        .filter(|entry| {
            let name = entry.file_name();
            name.to_str()
                .and_then(|name| name.strip_suffix(".png"))
                .map(|hash| hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()))
                .unwrap_or(false)
        })
        .filter_map(|entry| Some((entry.path(), entry.metadata().ok()?.modified().ok()?)))
        .collect();
    cached.sort_by_key(|(_, modified)| *modified);
    for (old_path, modified) in cached.iter().take(cached.len().saturating_sub(128)) {
        if !recent.contains_key(old_path)
            && modified
                .elapsed()
                .map(|age| age.as_secs() > 60)
                .unwrap_or(false)
        {
            let _ = std::fs::remove_file(old_path);
        }
    }
    Some(path)
}

#[tauri::command]
pub async fn show_desktop_notification(
    app: tauri::AppHandle,
    title: String,
    body: String,
    target: NotificationTarget,
    avatar_png: Option<Vec<u8>>,
) -> Result<bool, String> {
    if suppress_while_focused(&app, &target) {
        return Ok(false);
    }
    #[cfg(target_os = "windows")]
    {
        use tauri::Emitter;

        tauri::async_runtime::spawn_blocking(move || {
            use tauri_winrt_notification::{Duration, IconCrop, Toast};

            register_notification_identity(&app)?;
            let avatar = avatar_png
                .as_deref()
                .and_then(|bytes| cache_avatar(&app, bytes));
            if suppress_while_focused(&app, &target) {
                return Ok(false);
            }
            let make_toast = || {
                let click_app = app.clone();
                let click_target = target.clone();
                Toast::new(&app.config().identifier)
                    .title(&title)
                    .text1(&body)
                    .duration(Duration::Short)
                    .sound(None)
                    .on_activated(move |_| {
                        crate::tray::show_main_window(&click_app);
                        let _ = click_app.emit_to(
                            "main",
                            "desktop-notification-clicked",
                            &click_target,
                        );
                        Ok(())
                    })
            };
            if let Some(path) = avatar {
                if make_toast()
                    .icon(&path, IconCrop::Circular, "Sender profile picture")
                    .show()
                    .is_ok()
                {
                    return Ok(true);
                }
            }
            // Image loading/caching failure must not prevent a text notification.
            if suppress_while_focused(&app, &target) {
                return Ok(false);
            }
            make_toast().show().map(|_| true).map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    #[cfg(not(target_os = "windows"))]
    {
        use tauri_plugin_notification::NotificationExt;
        let _ = (target, avatar_png);
        app.notification()
            .builder()
            .title(title)
            .body(body)
            .show()
            .map(|_| true)
            .map_err(|e| e.to_string())
    }
}

fn suppress_while_focused(app: &tauri::AppHandle, target: &NotificationTarget) -> bool {
    use tauri::Manager;

    if matches!(target, NotificationTarget::Test) {
        return false;
    }
    app.get_webview_window("main").map_or(false, |window| {
        window.is_visible().unwrap_or(false)
            && !window.is_minimized().unwrap_or(false)
            && window.is_focused().unwrap_or(false)
    })
}
