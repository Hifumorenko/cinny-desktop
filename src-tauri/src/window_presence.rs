use std::{
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager, WebviewWindow};

const AWAY_AFTER: Duration = Duration::from_secs(5 * 60);

#[derive(Clone, Copy, Default, PartialEq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
enum Presence {
    #[default]
    Online,
    Unavailable,
}

#[derive(Clone, Copy, Default, serde::Serialize)]
pub struct Snapshot {
    revision: u64,
    presence: Presence,
}

#[derive(Default)]
struct Activity {
    in_tray: bool,
    minimized_since: Option<Instant>,
    snapshot: Snapshot,
}

impl Activity {
    fn snapshot(&mut self, now: Instant) -> Snapshot {
        let away = self.in_tray
            || self
                .minimized_since
                .map_or(false, |since| now.duration_since(since) >= AWAY_AFTER);
        let presence = if away {
            Presence::Unavailable
        } else {
            Presence::Online
        };
        if presence != self.snapshot.presence {
            self.snapshot.revision += 1;
            self.snapshot.presence = presence;
        }
        self.snapshot
    }

    fn minimized(&mut self, minimized: bool, now: Instant) {
        if self.in_tray || !minimized {
            self.minimized_since = None;
        } else {
            self.minimized_since.get_or_insert(now);
        }
    }
}

#[derive(Default)]
pub struct WindowPresence(Mutex<Activity>);

fn update(window: &WebviewWindow, change: impl FnOnce(&mut Activity, Instant)) {
    let state = window.state::<WindowPresence>();
    let snapshot = {
        let Ok(mut activity) = state.0.lock() else {
            return;
        };
        let revision = activity.snapshot.revision;
        let now = Instant::now();
        change(&mut activity, now);
        let snapshot = activity.snapshot(now);
        (snapshot.revision != revision).then(|| snapshot)
    };
    if let Some(snapshot) = snapshot {
        let _ = window.emit("desktop-presence-changed", snapshot);
    }
}

pub fn set_in_tray(window: &WebviewWindow, in_tray: bool) {
    update(window, |activity, _| {
        activity.in_tray = in_tray;
        activity.minimized_since = None;
    });
}

pub fn observe_window(window: &WebviewWindow) {
    // Query outside the state lock: native window queries may use the UI thread.
    if let Ok(minimized) = window.is_minimized() {
        update(window, |activity, now| activity.minimized(minimized, now));
    }
}

pub fn start(app: tauri::AppHandle) {
    std::thread::spawn(move || loop {
        let Some(window) = app.get_webview_window("main") else {
            break;
        };
        observe_window(&window);
        std::thread::sleep(Duration::from_secs(1));
    });
}

#[tauri::command]
pub fn get_desktop_presence(state: tauri::State<'_, WindowPresence>) -> Result<Snapshot, String> {
    state
        .0
        .lock()
        .map(|mut activity| activity.snapshot(Instant::now()))
        .map_err(|_| "Could not read desktop presence".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn away(snapshot: Snapshot) -> bool {
        snapshot.presence == Presence::Unavailable
    }

    #[test]
    fn minimize_waits_five_minutes_without_resetting_on_repeated_events() {
        let mut activity = Activity::default();
        let now = Instant::now();
        activity.minimized(true, now);
        assert!(!away(
            activity.snapshot(now + AWAY_AFTER - Duration::from_millis(1))
        ));
        activity.minimized(true, now + Duration::from_secs(100));
        assert!(away(activity.snapshot(now + AWAY_AFTER)));
        activity.minimized(false, now + AWAY_AFTER);
        assert!(!away(activity.snapshot(now + AWAY_AFTER)));
    }

    #[test]
    fn restoring_before_deadline_cancels_and_reminimizing_starts_a_new_timer() {
        let mut activity = Activity::default();
        let now = Instant::now();
        activity.minimized(true, now);
        activity.minimized(false, now + Duration::from_secs(200));
        assert!(!away(activity.snapshot(now + AWAY_AFTER)));
        activity.minimized(true, now + AWAY_AFTER);
        assert!(!away(
            activity.snapshot(now + AWAY_AFTER + Duration::from_secs(1))
        ));
        assert!(away(activity.snapshot(now + AWAY_AFTER * 2)));
    }

    #[test]
    fn tray_is_immediately_away_and_restore_is_online() {
        let mut activity = Activity {
            in_tray: true,
            ..Activity::default()
        };
        let now = Instant::now();
        let hidden = activity.snapshot(now);
        assert!(away(hidden));
        activity.minimized(false, now);
        assert_eq!(activity.snapshot(now).revision, hidden.revision);
        assert!(away(activity.snapshot(now)));
        activity.in_tray = false;
        let restored = activity.snapshot(now);
        assert!(!away(restored));
        assert!(restored.revision > hidden.revision);
    }
}
