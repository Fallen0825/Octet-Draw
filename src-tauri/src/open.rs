// Files Octet is asked to open (REL-14, CH-6): from Finder, Explorer or the Dock (a double-click, Open With, a file
// dropped on the app's icon), as arguments on the command line, from a second launch on Windows and Linux (which gives
// its files to the Octet that runs, and ends), or dropped on the window. The system names them, never the page: the
// page takes them with take_opened, which reads only these paths, with Open's checks.
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State, Window};

use crate::{recent, Picked, MAX_BYTES};

const MOST: usize = 20; // files kept waiting at one time
const DRAWINGS: [&str; 4] = ["oct", "cdxml", "cdx", "octet"];
const PICTURES: [&str; 8] = ["png", "jpg", "jpeg", "gif", "bmp", "tif", "tiff", "webp"];

// The files waiting for a page, each with the window it was dropped on and the point there (CSS pixels), if it was.
#[derive(Default)]
pub struct Waiting(Mutex<Vec<(PathBuf, Option<(String, f64, f64)>)>>);

// Files to open, or (with `at`) files dropped on a window. Files to open go to Home, which opens each drawing in a
// window of its own; Home comes back if it was closed. A dropped file goes to the window it was dropped on.
pub fn offer(app: &AppHandle, paths: impl IntoIterator<Item = PathBuf>, at: Option<(String, f64, f64)>) {
    // Files, and the folders of Bruker's processed spectra.
    let paths: Vec<PathBuf> = paths.into_iter().filter(|p| p.is_file() || crate::spectrum::bruker_folder(p).is_some()).take(MOST).collect();
    if paths.is_empty() {
        return;
    }
    {
        let waiting = app.state::<Waiting>();
        let mut list = waiting.0.lock().unwrap();
        list.extend(paths.into_iter().map(|p| (p, at.clone())));
        let over = list.len().saturating_sub(MOST);
        list.drain(..over);
    }
    match at {
        Some((label, ..)) => {
            let _ = app.emit_to(label, "opened", ());
        }
        None => {
            // Before Octet's first window is there (files named at launch), Home takes them when it starts.
            // Off the main thread: making Home again there deadlocks on Windows.
            if !app.webview_windows().is_empty() {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = crate::windows::home(&app);
                });
            }
            let _ = app.emit_to("main", "opened", ());
        }
    }
}

// The files a launch names: its arguments after the program's own path, read from `cwd`. Options ("-psn_…") are not files.
pub fn from_args(args: impl IntoIterator<Item = OsString>, cwd: &Path) -> Vec<PathBuf> {
    args.into_iter().skip(1).filter(|a| !a.to_string_lossy().starts_with('-')).map(|a| cwd.join(a)).collect()
}

#[derive(Serialize)]
pub struct OpenedFile {
    name: String,         // the file's name
    path: Option<String>, // set for a drawing Octet can save over (.oct, .cdxml, .cdx, .octet) that opens
    text: Option<String>,
    picture: Option<String>, // a picture's bytes in base64
    error: Option<String>,
    at: Option<(f64, f64)>, // where it was dropped
}

// The files waiting, read as text. insert: the page puts dropped files into the drawing that is open; otherwise they
// open. A drawing that opens can be saved over, as one picked in Open can, and goes on Home's list of recent drawings.
// project: the files were dropped on a project on Home. Each drawing dropped is only checked (it is there, 64 MB at
// most) and named, so the page can put it in the project (home.rs); it does not open. A picture comes back as a
// picture, which the page makes the project's thumbnail (src/thumbs.ts). Other files are refused.
#[tauri::command]
pub async fn take_opened(app: AppHandle, window: Window, picked: State<'_, Picked>, insert: bool, project: Option<bool>) -> Result<Vec<OpenedFile>, String> {
    // This window's: the files dropped on it, and on Home the files to open.
    let label = window.label();
    let list: Vec<_> = {
        let state = app.state::<Waiting>();
        let mut waiting = state.0.lock().unwrap();
        let (mine, rest): (Vec<_>, Vec<_>) = std::mem::take(&mut *waiting).into_iter().partition(|(_, at)| match at {
            Some((l, ..)) => l == label,
            None => label == "main",
        });
        *waiting = rest;
        mine
    };
    Ok(list
        .into_iter()
        .map(|(path, at)| {
            let at = at.map(|(_, x, y)| (x, y));
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let ext = path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).unwrap_or_default();
            if project == Some(true) && at.is_some() && !PICTURES.contains(&ext.as_str()) {
                let error = match fs::metadata(&path) {
                    _ if !DRAWINGS.contains(&ext.as_str()) => Some(format!("{name} is not a drawing. A folder holds Octet and ChemDraw drawings.")),
                    Err(_) => Some("This file is not where it was. It was moved or deleted.".into()),
                    Ok(m) if m.len() > MAX_BYTES => Some("This file is larger than 64 MB. Octet cannot open it.".into()),
                    Ok(_) => None,
                };
                if error.is_none() {
                    picked.0.lock().unwrap().insert(path.clone());
                }
                return OpenedFile { name, path: error.is_none().then(|| path.to_string_lossy().into_owned()), text: None, picture: None, error, at };
            }
            // A measured spectrum (JCAMP-DX, or a Bruker folder or its 1r file): the page puts it in the drawing.
            let bruker = crate::spectrum::bruker_folder(&path);
            if bruker.is_some() || crate::spectrum::is_jcamp(&path) {
                let read = match &bruker { Some(d) => crate::spectrum::read_bruker(d), None => crate::spectrum::read_jcamp(&path, &name) };
                let (text, error) = match read { Ok(t) => (Some(t), None), Err(e) => (None, Some(e)) };
                return OpenedFile { name, path: None, text, picture: None, error, at };
            }
            if PICTURES.contains(&ext.as_str()) {
                let (picture, error) = match picture(&path, &name) {
                    Ok(b) => (Some(b), None),
                    Err(e) => (None, Some(e)),
                };
                return OpenedFile { name, path: None, text: None, picture, error, at };
            }
            let read = read(&path, &name);
            let drawing = DRAWINGS.contains(&ext.as_str());
            let kept = read.is_ok() && drawing && (at.is_none() || !insert);
            if kept {
                picked.0.lock().unwrap().insert(path.clone());
                recent::note(&app, &path);
            }
            let (text, error) = match read {
                Ok(t) => (Some(t), None),
                Err(e) => (None, Some(e)),
            };
            OpenedFile { name, path: kept.then(|| path.to_string_lossy().into_owned()), text, picture: None, error, at }
        })
        .collect())
}

// A file's text (a CDX file's bytes in base64), with Open's limits: 64 MB at most, and text (UTF-8) or CDX only.
fn read(path: &Path, name: &str) -> Result<String, String> {
    match fs::metadata(path) {
        Err(_) => Err("This file is not where it was. It was moved or deleted.".into()),
        Ok(m) if m.len() > MAX_BYTES => Err("This file is larger than 64 MB. Octet cannot open it.".into()),
        Ok(_) => crate::read_text_or_cdx(path).map_err(|_| format!("Octet cannot open {name}. It opens Octet and ChemDraw drawings (CDX and CDXML), ChemDraw template files, MOL, SDF, RXN, SMILES and InChI files, JCAMP-DX spectra, and pictures.")),
    }
}

// A picture file's bytes in base64, with Open's limit of 64 MB. The page checks what they are, decodes them and keeps
// 12 MB of pictures in a drawing at most (src/picture.ts).
fn picture(path: &Path, name: &str) -> Result<String, String> {
    use base64::Engine;
    match fs::metadata(path) {
        Err(_) => Err("This file is not where it was. It was moved or deleted.".into()),
        Ok(m) if m.len() > MAX_BYTES => Err(format!("Octet cannot insert {name}. It is larger than 64 MB.")),
        Ok(_) => fs::read(path).map(|b| base64::engine::general_purpose::STANDARD.encode(b)).map_err(|_| format!("Octet cannot read {name}.")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_arguments_are_files() {
        let cwd = Path::new("/Users/me/Desktop");
        let args = ["/Applications/Octet.app/Contents/MacOS/octet", "-psn_0_12345", "aspirin.oct", "/tmp/b.cdxml"].map(OsString::from);
        assert_eq!(from_args(args, cwd), [PathBuf::from("/Users/me/Desktop/aspirin.oct"), PathBuf::from("/tmp/b.cdxml")]);
        assert!(from_args([OsString::from("octet")], cwd).is_empty());
    }

    #[test]
    fn files_are_read_with_opens_limits() {
        let dir = std::env::temp_dir().join(format!("octet-open-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let (text, png, big, cdx) = (dir.join("a.mol"), dir.join("b.png"), dir.join("c.cdxml"), dir.join("d.cdx"));
        fs::write(&text, "CCO\n").unwrap();
        fs::write(&cdx, b"VjCD0100\x04\x03\x02\x01\xff").unwrap();
        fs::write(&png, [0x89, b'P', b'N', b'G', 0xff, 0xfe]).unwrap();
        fs::File::create(&big).unwrap().set_len(MAX_BYTES + 1).unwrap();
        assert_eq!(read(&text, "a.mol").as_deref(), Ok("CCO\n"));
        assert!(read(&png, "b.png").unwrap_err().starts_with("Octet cannot open b.png."));
        // A CDX file is binary: the page gets its bytes in base64.
        assert_eq!(read(&cdx, "d.cdx").as_deref(), Ok("VmpDRDAxMDAEAwIB/w=="));
        assert!(read(&big, "c.cdxml").unwrap_err().contains("64 MB"));
        assert!(read(&dir.join("gone.oct"), "gone.oct").unwrap_err().contains("moved or deleted"));
        // A picture comes as its bytes, with the same limit.
        assert_eq!(picture(&png, "b.png").as_deref(), Ok("iVBOR//+"));
        assert!(picture(&big, "c.png").unwrap_err().contains("64 MB"));
        fs::remove_dir_all(&dir).unwrap();
    }
}
