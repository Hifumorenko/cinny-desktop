use std::{path::Path, sync::Mutex};
use tauri::{AppHandle, Manager};

#[derive(Default)]
pub struct CloseBehavior(Mutex<bool>);

impl CloseBehavior {
    pub fn load(app: &AppHandle) -> Self {
        let value = (|| -> Result<bool, Box<dyn std::error::Error>> {
            let path = app.path().app_config_dir()?.join("quit-on-close.json");
            read_preference(&path)
        })();
        match value {
            Ok(value) => Self(Mutex::new(value)),
            Err(_) => {
                eprintln!("Could not load close behavior; using minimize to tray");
                Self::default()
            }
        }
    }

    pub fn should_quit(&self) -> bool {
        self.0.lock().map(|value| *value).unwrap_or(false)
    }

    fn save(&self, path: &Path, enabled: bool) -> Result<(), String> {
        let mut value = self
            .0
            .lock()
            .map_err(|_| "Could not update close behavior")?;
        let directory = path.parent().ok_or("Could not locate desktop settings")?;
        std::fs::create_dir_all(directory).map_err(|_| "Could not save desktop settings")?;
        // Replace only after the complete new value has been written. A failed
        // write must preserve the previous preference on disk and in memory.
        let temporary = path.with_extension("json.tmp");
        let result = std::fs::write(&temporary, if enabled { "true" } else { "false" })
            .and_then(|()| std::fs::rename(&temporary, path));
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary);
            return Err("Could not save desktop settings".into());
        }
        *value = enabled;
        Ok(())
    }
}

fn read_preference(path: &Path) -> Result<bool, Box<dyn std::error::Error>> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

#[tauri::command]
pub fn get_quit_on_close(state: tauri::State<'_, CloseBehavior>) -> bool {
    state.should_quit()
}

#[tauri::command]
pub fn set_quit_on_close(
    app: AppHandle,
    enabled: bool,
    state: tauri::State<'_, CloseBehavior>,
) -> Result<(), String> {
    let directory = app
        .path()
        .app_config_dir()
        .map_err(|_| "Could not locate desktop settings")?;
    state.save(&directory.join("quit-on-close.json"), enabled)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let directory = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("target/close-behavior-tests")
                .join(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                        .to_string(),
                );
            std::fs::create_dir_all(&directory).unwrap();
            Self(directory)
        }

        fn path(&self) -> PathBuf {
            self.0.join("quit-on-close.json")
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/close-behavior-tests");
            if let (Ok(root), Ok(directory)) = (root.canonicalize(), self.0.canonicalize()) {
                if directory.starts_with(&root) && directory != root {
                    let _ = std::fs::remove_dir_all(directory);
                }
            }
        }
    }

    #[test]
    fn missing_preference_defaults_to_off() {
        let fixture = Fixture::new();
        assert!(!CloseBehavior::default().should_quit());
        assert!(!read_preference(&fixture.path()).unwrap());
    }

    #[test]
    fn preference_survives_restart_and_can_be_disabled() {
        let fixture = Fixture::new();
        let state = CloseBehavior::default();
        state.save(&fixture.path(), true).unwrap();
        assert!(state.should_quit());
        let restored = CloseBehavior(Mutex::new(read_preference(&fixture.path()).unwrap()));
        assert!(restored.should_quit());
        restored.save(&fixture.path(), false).unwrap();
        assert!(!restored.should_quit());
        assert!(!read_preference(&fixture.path()).unwrap());
    }

    #[test]
    fn failed_save_preserves_the_previous_preference() {
        let fixture = Fixture::new();
        let path = fixture.path();
        let state = CloseBehavior::default();
        state.save(&path, true).unwrap();
        std::fs::create_dir(path.with_extension("json.tmp")).unwrap();
        assert!(state.save(&path, false).is_err());
        assert!(state.should_quit());
        assert!(read_preference(&path).unwrap());
    }
}
