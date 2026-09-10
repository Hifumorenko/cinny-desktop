#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

// mod menu;

use std::{collections::HashSet, sync::{Arc, RwLock}};
use tauri::{
    webview::{NewWindowResponse, WebviewWindowBuilder},
    Emitter, Manager, WebviewUrl,
};
#[cfg(target_os = "macos")]
use tauri::TitleBarStyle;
use tauri_plugin_opener::OpenerExt;

#[cfg(feature = "updater")]
use tauri_plugin_updater::UpdaterExt;
#[cfg(feature = "updater")]
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

#[derive(Clone, Default)]
struct PushToTalkBinds(Arc<RwLock<HashSet<String>>>);

#[derive(Clone, serde::Serialize)]
struct PushToTalkInput { code: String, pressed: bool }

#[tauri::command]
fn set_push_to_talk_binds(binds: Vec<String>, state: tauri::State<'_, PushToTalkBinds>) {
    if let Ok(mut current) = state.0.write() { *current = binds.into_iter().collect(); }
}

#[cfg(not(target_os = "windows"))]
fn ptt_input(event: &rdev::EventType) -> Option<PushToTalkInput> {
    use rdev::{Button, EventType, Key};
    let (code, pressed) = match event {
        EventType::KeyPress(key) | EventType::KeyRelease(key) => {
            let code = match key {
                Key::Alt => "AltLeft".into(), Key::AltGr => "AltRight".into(),
                Key::DownArrow => "ArrowDown".into(), Key::LeftArrow => "ArrowLeft".into(),
                Key::RightArrow => "ArrowRight".into(), Key::UpArrow => "ArrowUp".into(),
                Key::Return => "Enter".into(),
                Key::Num0 => "Digit0".into(), Key::Num1 => "Digit1".into(),
                Key::Num2 => "Digit2".into(), Key::Num3 => "Digit3".into(),
                Key::Num4 => "Digit4".into(), Key::Num5 => "Digit5".into(),
                Key::Num6 => "Digit6".into(), Key::Num7 => "Digit7".into(),
                Key::Num8 => "Digit8".into(), Key::Num9 => "Digit9".into(),
                Key::Unknown(_) => return None, key => format!("{:?}", key),
            };
            (code, matches!(event, EventType::KeyPress(_)))
        }
        EventType::ButtonPress(button) | EventType::ButtonRelease(button) => {
            let number = match button {
                Button::Left => 0, Button::Middle => 1, Button::Right => 2,
                Button::Unknown(number) => {
                    #[cfg(target_os = "windows")] { u32::from(*number) + 2 }
                    #[cfg(target_os = "linux")] { u32::from(*number).saturating_sub(5) }
                    #[cfg(target_os = "macos")] { u32::from(*number) }
                }
            };
            (format!("Mouse{}", number), matches!(event, EventType::ButtonPress(_)))
        }
        _ => return None,
    };
    Some(PushToTalkInput { code, pressed })
}

#[cfg(target_os = "windows")]
fn poll_windows_ptt(handle: tauri::AppHandle) {
    use std::{collections::HashMap, thread, time::Duration};
    use winapi::um::winuser::{
        GetAsyncKeyState, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_RCONTROL, VK_RMENU, VK_RSHIFT,
        VK_XBUTTON1, VK_XBUTTON2,
    };

    let mut keys = vec![
        ("ShiftLeft", VK_LSHIFT),
        ("ShiftRight", VK_RSHIFT),
        ("ControlLeft", VK_LCONTROL),
        ("ControlRight", VK_RCONTROL),
        ("AltLeft", VK_LMENU),
        ("AltRight", VK_RMENU),
        // DOM button 3/4 are the physical Mouse 4/5 buttons.
        ("Mouse3", VK_XBUTTON1),
        ("Mouse4", VK_XBUTTON2),
    ];
    for letter in b'A'..=b'Z' {
        keys.push((Box::leak(format!("Key{}", letter as char).into_boxed_str()), letter as i32));
    }
    for digit in b'0'..=b'9' {
        keys.push((Box::leak(format!("Digit{}", digit as char).into_boxed_str()), digit as i32));
    }
    let mut states = HashMap::<&str, bool>::new();

    loop {
        for &(code, virtual_key) in &keys {
            let pressed = unsafe { GetAsyncKeyState(virtual_key) < 0 };
            let previous = states.insert(code, pressed).unwrap_or(false);
            if pressed != previous {
                let _ = handle.emit_to(
                    "main",
                    "push-to-talk-input",
                    PushToTalkInput {
                        code: code.into(),
                        pressed,
                    },
                );
            }
        }
        thread::sleep(Duration::from_millis(8));
    }
}

pub fn run() {
    
    for key in ["NO_PROXY", "no_proxy"] {
        let current_val = std::env::var(key).unwrap_or_default();
        if !current_val.contains("localhost") {
            let new_val = if current_val.is_empty() {
                "localhost,127.0.0.1".to_string()
            } else {
                format!("{},localhost,127.0.0.1", current_val)
            };
            std::env::set_var(key, new_val);
        }
    }

    let port: u16 = 44548;
    let context = tauri::generate_context!();
    #[cfg(feature = "updater")]
    let mut builder = tauri::Builder::default();
    #[cfg(not(feature = "updater"))]
    let mut builder = tauri::Builder::default();

    // This must be the first registered plugin so concurrent launches are
    // intercepted before the rest of the application starts.
    builder = builder.plugin(tauri_plugin_single_instance::init(
        |app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        },
    ));

    // #[cfg(target_os = "macos")]
    // {
    //     builder = builder.menu(menu::menu());
    // }

    #[cfg(feature = "updater")]
    {
        builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
    }

    builder
        .manage(PushToTalkBinds::default())
        .invoke_handler(tauri::generate_handler![set_push_to_talk_binds])
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_localhost::Builder::new(port).build())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(move |app| {
            let handle = app.handle().clone();
            #[cfg(not(target_os = "windows"))]
            let binds = app.state::<PushToTalkBinds>().inner().clone();
            #[cfg(target_os = "windows")]
            std::thread::spawn(move || poll_windows_ptt(handle));

            #[cfg(not(target_os = "windows"))]
            std::thread::spawn(move || {
                let _ = rdev::listen(move |event| {
                    let Some(input) = ptt_input(&event.event_type) else { return };
                    if binds.0.read().map(|set| set.contains(&input.code)).unwrap_or(false) {
                        let _ = handle.emit_to("main", "push-to-talk-input", input);
                    }
                });
            });

            #[cfg(feature = "updater")]
            {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    let updater = match handle.updater() {
                        Ok(u) => u,
                        Err(e) => {
                            eprintln!("Updater not available: {}", e);
                            return;
                        }
                    };
                    if let Ok(Some(update)) = updater.check().await {
                        let version = update.version.clone();

                        let should_update = handle
                            .dialog()
                            .message(format!(
                                "Version {} is available.\n\nWould you like to update now?",
                                version
                            ))
                            .title("Update Available")
                            .kind(MessageDialogKind::Info)
                            .buttons(MessageDialogButtons::YesNo)
                            .blocking_show();

                        if should_update {
                            if update.download_and_install(|_, _| {}, || {}).await.is_ok() {
                                handle.restart();
                            }
                        }
                    }
                });
            }

            // Dev: use devUrl from tauri.conf.json (http://localhost:8080) to support HMR
            #[cfg(debug_assertions)]
            let window_url = WebviewUrl::App(Default::default());

            // Release: tauri-plugin-localhost serves bundled frontend assets on this port.
            // Keep this origin stable because Cinny's login and crypto stores are origin-scoped.
            #[cfg(not(debug_assertions))]
            let window_url = {
                let url = format!("http://localhost:{}", port).parse().unwrap();
                WebviewUrl::External(url)
            };

            let app_handle = app.handle().clone();
            let window_builder = WebviewWindowBuilder::new(app, "main".to_string(), window_url)
                .title("Cinny")
                .disable_drag_drop_handler()
                .on_new_window(move |url, _features| {
                    let _ = app_handle.opener().open_url(url.as_str(), None::<&str>);
                    NewWindowResponse::Deny
                });

            #[cfg(target_os = "macos")]
            let window_builder = window_builder.title_bar_style(TitleBarStyle::Transparent);
            
            window_builder.build()?;
            Ok(())
        })
        .run(context)
        .expect("error while building tauri application");
}
