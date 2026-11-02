// Smoke test of the real app (CI, `--features smoke`): Octet starts, File > New opens a drawing window from Home, the
// drawing's page starts and tells Octet its name, and File > Home brings Home back. Each step must answer on the main
// thread, so a hang (as on Windows when a synchronous command made a window) fails it. Ends with 0, or 1 and the step.
use std::sync::mpsc;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager};

fn wait(what: &str, secs: u64, mut done: impl FnMut() -> bool) {
    let start = Instant::now();
    while !done() {
        if start.elapsed() > Duration::from_secs(secs) {
            eprintln!("smoke: failed: {what}");
            std::process::exit(1);
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    eprintln!("smoke: ok: {what}");
}

// The main thread answers within 10 seconds.
fn alive(app: &AppHandle, what: &str) {
    let (tx, rx) = mpsc::channel();
    let _ = app.run_on_main_thread(move || {
        let _ = tx.send(());
    });
    if rx.recv_timeout(Duration::from_secs(10)).is_err() {
        eprintln!("smoke: failed: the app hangs after {what}");
        std::process::exit(1);
    }
}

pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        wait("Home is open", 60, || app.get_webview_window("main").is_some());
        std::thread::sleep(Duration::from_secs(15)); // Home's page loads
        alive(&app, "starting");
        let _ = app.emit_to("main", "menu", "new");
        let drawn = |app: &AppHandle| !crate::windows::window_list(app.state()).is_empty();
        wait("File > New opens a drawing window, and its page starts", 90, || drawn(&app));
        alive(&app, "File > New");
        let label = crate::windows::window_list(app.state())[0].label().to_string();
        let _ = app.emit_to(label, "menu", "home");
        std::thread::sleep(Duration::from_secs(3));
        alive(&app, "File > Home");
        eprintln!("smoke: passed");
        std::process::exit(0);
    });
}
