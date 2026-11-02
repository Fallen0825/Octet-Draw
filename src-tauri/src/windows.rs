// Drawing windows. Home is the window "main"; each drawing has a window of its own ("draw-1", "draw-2" and on), as
// ChemDraw has a window for each document. Home (or a drawing window, for New and Open) asks for a new window with
// what it should open, and the new page takes that once it starts (window_start). Each drawing window tells this side
// its name, file and whether it has unsaved changes, for Home's list of open windows.
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder, Window};

use crate::MAX_BYTES;

static NEXT: AtomicU64 = AtomicU64::new(1);
// The last of Octet's windows in front, for menu commands while another window (Reaxys) is in front.
static LAST: Mutex<String> = Mutex::new(String::new());

#[derive(Clone, Serialize, Deserialize)]
pub struct Info {
    label: String,
    name: String,
    path: Option<String>,
    edited: bool,
    picture: Option<String>, // the drawing as an SVG, as a recent drawing's (400,000 characters at most)
}

#[cfg(feature = "smoke")]
impl Info {
    pub fn label(&self) -> &str {
        &self.label
    }
}

// What each new window is to open, until its page takes it, and what each drawing window shows.
#[derive(Default)]
pub struct Windows {
    starts: Mutex<HashMap<String, serde_json::Value>>,
    open: Mutex<Vec<Info>>,
}

// Octet's own windows: Home and the drawing windows (not the Reaxys window).
pub fn ours(label: &str) -> bool {
    label == "main" || label.starts_with("draw-")
}

pub fn focused(label: &str) {
    if ours(label) {
        *LAST.lock().unwrap() = label.into();
    }
}

// The window that gets menu commands: the one in front, else the last of Octet's in front, else Home, else the
// oldest drawing window.
pub fn front(app: &AppHandle) -> Option<String> {
    let windows = app.webview_windows();
    let mut labels: Vec<&String> = windows.keys().filter(|l| ours(l)).collect();
    labels.sort_by_key(|l| (l.as_str() != "main", l.trim_start_matches("draw-").parse::<u64>().unwrap_or(0)));
    let last = LAST.lock().unwrap().clone();
    labels
        .iter()
        .find(|l| windows[l.as_str()].is_focused().unwrap_or(false))
        .or(labels.iter().find(|l| l.as_str() == last))
        .or(labels.first())
        .map(|l| l.to_string())
}

fn changed(app: &AppHandle) {
    let list = app.state::<Windows>().open.lock().unwrap().clone();
    let _ = app.emit("windows", list);
}

// A new drawing window, a little below and right of the window in front, with what it opens (64 MB at most).
// The commands that make or show windows are async: on Windows, making a window in a synchronous command (which runs
// on the main thread) deadlocks, and the app hangs with a white window.
#[tauri::command]
pub async fn open_window(app: AppHandle, windows: State<'_, Windows>, start: serde_json::Value) -> Result<(), String> {
    if start.to_string().len() as u64 > MAX_BYTES + 4096 {
        return Err("This drawing is larger than 64 MB. Octet cannot open it.".into());
    }
    let label = format!("draw-{}", NEXT.fetch_add(1, Ordering::Relaxed));
    windows.starts.lock().unwrap().insert(label.clone(), start);
    let mut builder = WebviewWindowBuilder::new(&app, &label, WebviewUrl::App("index.html?window".into()))
        .title("Octet Draw")
        .inner_size(1200.0, 800.0)
        .min_inner_size(800.0, 620.0);
    let from = front(&app).and_then(|l| app.get_webview_window(&l));
    if let Some((pos, scale)) = from.and_then(|w| Some((w.outer_position().ok()?, w.scale_factor().ok()?))) {
        let pos = pos.to_logical::<f64>(scale);
        builder = builder.position(pos.x + 28.0, pos.y + 28.0);
    } else {
        builder = builder.center();
    }
    let window = builder.build().map_err(|e| {
        windows.starts.lock().unwrap().remove(&label);
        e.to_string()
    })?;
    #[cfg(windows)]
    crate::access_keys::install(&window);
    let _ = window.set_focus();
    Ok(())
}

// What this window opens: once, then nothing.
#[tauri::command]
pub fn window_start(window: Window, windows: State<'_, Windows>) -> Option<serde_json::Value> {
    windows.starts.lock().unwrap().remove(window.label())
}

// A drawing window's name, file and unsaved changes, for Home, and its picture when it sends one (else the last).
#[tauri::command]
pub fn window_info(app: AppHandle, window: Window, windows: State<'_, Windows>, name: String, path: Option<String>, edited: bool, picture: Option<String>) {
    if !window.label().starts_with("draw-") {
        return;
    }
    let picture = picture.filter(|p| p.len() <= 400_000);
    let name = name.chars().filter(|c| !c.is_control()).take(200).collect();
    {
        let mut open = windows.open.lock().unwrap();
        match open.iter_mut().find(|i| i.label == window.label()) {
            Some(i) => {
                let picture = picture.or(i.picture.take());
                *i = Info { label: window.label().into(), name, path, edited, picture };
            }
            None => open.push(Info { label: window.label().into(), name, path, edited, picture }),
        }
    }
    changed(&app);
}

#[tauri::command]
pub fn window_list(windows: State<'_, Windows>) -> Vec<Info> {
    windows.open.lock().unwrap().clone()
}

// A window from Home's list, in front.
#[tauri::command]
pub async fn window_show(app: AppHandle, label: String) -> Result<(), String> {
    let window = app.get_webview_window(&label).filter(|_| ours(&label)).ok_or("This window is closed.")?;
    let _ = window.unminimize();
    let _ = window.show();
    window.set_focus().map_err(|e| e.to_string())
}

// File > Home from a drawing window: Home in front, opened again if it was closed.
#[tauri::command]
pub async fn show_home(app: AppHandle) -> Result<(), String> {
    home(&app).map_err(|e| e.to_string())
}

pub fn home(app: &AppHandle) -> tauri::Result<()> {
    let window = match app.get_webview_window("main") {
        Some(w) => w,
        None => {
            let w = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("Octet Draw")
                .inner_size(1200.0, 800.0)
                .min_inner_size(800.0, 620.0)
                .center()
                .build()?;
            #[cfg(windows)]
            crate::access_keys::install(&w);
            w
        }
    };
    let _ = window.unminimize();
    window.show()?;
    window.set_focus()
}

// A closed window leaves Home's list.
pub fn closed(app: &AppHandle, label: &str) {
    let windows = app.state::<Windows>();
    windows.starts.lock().unwrap().remove(label);
    let before = {
        let mut open = windows.open.lock().unwrap();
        let n = open.len();
        open.retain(|i| i.label != label);
        n != open.len()
    };
    if before {
        changed(app);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn octets_windows_only() {
        assert!(ours("main") && ours("draw-3"));
        assert!(!ours("reaxys") && !ours("drawing"));
    }
}
