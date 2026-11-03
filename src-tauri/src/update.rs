// Updates (decision 0010). Octet never checks by itself: the page asks for a check only when the user said yes to
// automatic checks (on first launch, or later in Settings), or chose Check for Updates. Every update is signed, and
// the updater installs it only when the signature matches the public key in tauri.conf.json.
use std::sync::Mutex;

use tauri::{AppHandle, State};
use tauri_plugin_updater::Update;

// The update the last check found, kept for Install.
#[derive(Default)]
pub struct Found(Mutex<Option<Update>>);

// The newer version, or None when Octet is up to date. With no network the check fails, and the page says so only
// when the user asked. A check that gets no answer stops after 30 s.
#[tauri::command]
// Korean build: no update check. The original's updates are the English build, which would replace this one; a new
// Korean build comes from the fork's GitHub page instead. The page shows its own message for the error.
pub async fn update_check(_app: AppHandle, _found: State<'_, Found>) -> Result<Option<String>, String> {
    Err("The Korean build does not update itself.".into())
}

// Downloads the update found, checks its signature, installs it and starts Octet again. The page asks to save the
// drawing first. The download has no time limit, as a slow network needs longer. On Windows the installer closes
// Octet itself.
#[tauri::command]
pub async fn update_install(app: AppHandle, found: State<'_, Found>) -> Result<(), String> {
    let mut update = found.0.lock().unwrap().take().ok_or("Octet found no update to install.")?;
    update.timeout = None;
    update.download_and_install(|_, _| {}, || {}).await.map_err(|e| e.to_string())?;
    app.restart()
}
