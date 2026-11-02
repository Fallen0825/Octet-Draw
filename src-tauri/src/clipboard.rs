// Copy and paste with Word, PowerPoint, Keynote and ChemDraw, as ChemDraw's own clipboard works.
// Copy puts one item holding, in this order: a picture of the selection, ChemDraw's CDX (ChemDraw reads it first, so
// the whole drawing pastes into ChemDraw), CDXML, a molfile, SMILES, and Octet's own JSON for a lossless paste into
// Octet. Settings > Import and export > Copy switches the picture, CDX, CDXML, MOL and SMILES off (REL-19, CH-8);
// Octet's JSON always goes on. No plain text: ChemDraw reads text before pictures and would paste it as a caption,
// and Word pastes text before a picture.
// - macOS: the picture is a vector PDF with the CDXML in its catalog /Metadata (Office keeps it; ChemDraw and Octet
//   read the structure back out of it). The types are ChemDraw's (notes/clipboard-blackbox.md).
// - Windows (REL-3, CH-1): Office for Windows pastes no PDF, so the picture is an EMF (vector, as ChemDraw for Windows
//   puts a metafile), with the CDXML in a comment, and a bitmap for programs that read no metafile. No PNG: Office
//   would take it before the EMF and paste pixels. The other formats
//   have the names ChemDraw's own clipboard code uses there: "ChemDraw Interchange Format" (CDX), "CDXML", "MDLCT" (a
//   molfile as length-prefixed lines) and "SMILES".
// - Linux: the picture is a PNG and an SVG, and the chemistry has its MIME types (chemical/x-cdx and so on).
//
// The webview keeps a page's own clipboard types in a form of its own and shows a page only text, HTML and URLs from
// other apps, so the page cannot read these types itself: paste calls read_clipboard. Paste also reads a picture (a
// screenshot, a picture copied from a browser): PNG, else the system's bitmap (TIFF on the Mac, the bitmap on Windows).

pub const PDF: &str = "com.adobe.pdf";
// ChemDraw 26.1 declares the Revvity names too (its Info.plist), and the CambridgeSoft names for the same CDX and CDXML
// codes. ChemDraw asks macOS for the name of each code, so the name it reads can be any of them: copy writes and paste
// reads all three.
pub const CDX: [&str; 3] = ["com.perkinelmer.chemdraw.cdx-clipboard", "com.revvity.chemdraw.cdx-clipboard", "com.cambridgesoft.cdx"];
pub const CDXML: [&str; 3] = ["com.perkinelmer.cdxml", "com.revvity.cdxml", "com.cambridgesoft.cdxml"];
pub const MOLFILE: &str = "com.mdli.molfile";
pub const SMILES: &str = "org.opensmiles.smiles";
// Plain text, for Name to Structure (the name field opens with a name copied from elsewhere).
pub const TEXT: &str = "public.utf8-plain-text";
// Octet's own type. A UTI under the app's identifier: macOS refuses MIME-style names such as
// application/x-octet+json on the pasteboard. Windows and Linux use that name.
pub const OCTET: &str = "io.github.samchemman.octet.json";
// A copied file, on the Mac.
pub const FILE_URL: &str = "public.file-url";
// The Windows bitmap and text are standard formats (CF_DIB, CF_UNICODETEXT), not registered names.
pub const DIB: &str = "CF_DIB";
pub const UNICODE_TEXT: &str = "CF_UNICODETEXT";
pub const ENHMETAFILE: &str = "CF_ENHMETAFILE";

const MAX_TYPE: usize = 64 << 20;
const MAX_TEXT: usize = 64 << 10;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum System {
    Mac,
    Windows,
    Linux,
}
pub const HERE: System = if cfg!(target_os = "macos") { System::Mac } else if cfg!(windows) { System::Windows } else { System::Linux };

// What one copy can hold.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Pdf,
    Emf,
    Png,
    Bitmap,
    Svg,
    Cdx,
    Cdxml,
    Molfile,
    Smiles,
    Octet,
    Text,
    FileUrl, // read only: a copied file, for a spectrum (spectrum.rs)
}

// Each system's names for a kind, in the order copy writes them; paste reads the first one there. Empty: that system's
// clipboard does not have the kind.
fn names(s: System, k: Kind) -> &'static [&'static str] {
    use Kind::*;
    match s {
        System::Mac => match k {
            Pdf => &[PDF],
            Cdx => &CDX,
            Cdxml => &CDXML,
            Molfile => &[MOLFILE],
            Smiles => &[SMILES],
            Octet => &[OCTET],
            Text => &[TEXT],
            // Read only: copy puts a PDF on the Mac.
            Png => &["public.png"],
            Bitmap => &["public.tiff"],
            FileUrl => &[FILE_URL],
            Svg | Emf => &[],
        },
        System::Windows => match k {
            Emf => &[ENHMETAFILE],
            Png => &["PNG"],
            Bitmap => &[DIB],
            Cdx => &["ChemDraw Interchange Format"],
            Cdxml => &["CDXML"],
            Molfile => &["MDLCT"],
            Smiles => &["SMILES"],
            Octet => &["application/x-octet+json"],
            Text => &[UNICODE_TEXT],
            Pdf | Svg | FileUrl => &[],
        },
        System::Linux => match k {
            Png => &["image/png"],
            Svg => &["image/svg+xml"],
            Cdx => &["chemical/x-cdx"],
            Cdxml => &["chemical/x-cdxml"],
            Molfile => &["chemical/x-mdl-molfile"],
            Smiles => &["chemical/x-daylight-smiles"],
            Octet => &["application/x-octet+json"],
            Text => &["text/plain;charset=utf-8", "UTF8_STRING", "text/plain"],
            FileUrl => &["text/uri-list"],
            Pdf | Bitmap | Emf => &[],
        },
    }
}

// What paste found on the clipboard. Each is None when absent or unreadable.
#[derive(serde::Serialize, Default, Debug, PartialEq)]
pub struct Clip {
    octet: Option<String>,   // Octet's own JSON
    cdxml: Option<String>,   // CDXML: ChemDraw's CDXML type, or hidden in a PDF (Octet's, ChemDraw's, or either after Word)
    cdx: Option<String>,     // ChemDraw's CDX, base64
    molfile: Option<String>, // a molfile or RXN file, with LF line ends
    smiles: Option<String>,
    text: Option<String>,    // plain text of 64 KB or less
    picture: Option<String>, // a picture's file bytes (PNG, TIFF or BMP), base64; the page checks and decodes them
    spectrum: Option<(String, String)>, // a spectrum file copied, or its path pasted: its name and text (spectrum.rs)
}

// ChemDraw's form of a molfile on the clipboard (MDL's MDLCT): each line as one length byte, then the line, with no
// line ends. None if a line is longer than 255 bytes (V2000 lines never are).
pub fn prefixed_lines(text: &str) -> Option<Vec<u8>> {
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    let text = text.strip_suffix('\n').unwrap_or(&text);
    let mut out = Vec::with_capacity(text.len() + 1);
    for line in text.split('\n') {
        out.push(u8::try_from(line.len()).ok()?);
        out.extend_from_slice(line.as_bytes());
    }
    Some(out)
}

// A molfile from the clipboard: ChemDraw's length-prefixed lines, or ordinary text with CR, LF or CRLF.
pub fn molfile_text(b: &[u8]) -> Option<String> {
    let b = &b[..b.iter().rposition(|&c| c != 0).map_or(0, |k| k + 1)];
    if b.is_empty() {
        return None;
    }
    let mut lines = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let n = usize::from(b[i]);
        match b.get(i + 1..i + 1 + n) {
            Some(line) if !line.contains(&b'\r') && !line.contains(&b'\n') => lines.push(line),
            _ => {
                lines.clear();
                break;
            }
        }
        i += 1 + n;
    }
    let text = if lines.is_empty() {
        String::from_utf8_lossy(b).replace("\r\n", "\n").replace('\r', "\n")
    } else {
        String::from_utf8_lossy(&lines.join(&b'\n')).into_owned()
    };
    Some(if text.ends_with('\n') { text } else { text + "\n" })
}

// SMILES from the clipboard: ASCII, with or without ChemDraw's trailing NUL.
pub fn smiles_text(b: &[u8]) -> Option<String> {
    let s = std::str::from_utf8(b).ok()?.trim_matches(|c: char| c == '\0' || c.is_whitespace());
    (!s.is_empty() && s.is_ascii() && !s.contains('\0')).then(|| s.to_string())
}

// Text from the clipboard as UTF-8, without the NULs a Windows clipboard can add at the end.
fn text_of(b: Vec<u8>) -> Option<String> {
    let end = b.iter().rposition(|&c| c != 0).map_or(0, |k| k + 1);
    String::from_utf8(b[..end].to_vec()).ok()
}

// Plain text from the clipboard: UTF-16 on Windows, UTF-8 elsewhere, without NULs at the end.
fn plain_text(s: System, b: Vec<u8>) -> Option<String> {
    if s != System::Windows {
        return text_of(b);
    }
    let units: Vec<u16> = b.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    let end = units.iter().rposition(|&c| c != 0).map_or(0, |k| k + 1);
    String::from_utf16(&units[..end]).ok()
}

// The types for one copy on system `s`, in ChemDraw's order, as (name, bytes). svg: the picture (None: no picture).
// cdxml: hidden in the PDF (macOS), and also put on as its own type when `cdxml_type` is set. cdx: ChemDraw's binary
// drawing.
#[allow(clippy::too_many_arguments)]
fn types(s: System, svg: Option<&str>, cdxml: Option<&str>, cdxml_type: bool, cdx: Option<Vec<u8>>, molfile: Option<&str>, smiles: Option<&str>, octet_json: Option<String>) -> Result<Vec<(&'static str, Vec<u8>)>, String> {
    let mut out: Vec<(&'static str, Vec<u8>)> = Vec::new();
    let mut put = |k: Kind, b: Vec<u8>| out.extend(names(s, k).iter().map(|n| (*n, b.clone())));
    if let Some(svg) = svg {
        if s == System::Mac {
            put(Kind::Pdf, crate::export::make_pdf(svg, cdxml)?);
        } else if s == System::Windows {
            let tree = crate::export::tree(svg)?;
            put(Kind::Emf, crate::vector::make_emf(&tree, cdxml));
            if let (_, Some(b)) = crate::export::clipboard_pixels(&tree, true)? {
                put(Kind::Bitmap, b);
            }
        } else {
            let (png, bitmap) = crate::export::clipboard_pictures(svg, false)?;
            put(Kind::Png, png);
            if let Some(b) = bitmap {
                put(Kind::Bitmap, b);
            }
            if s == System::Linux {
                put(Kind::Svg, svg.as_bytes().to_vec());
            }
        }
    }
    if let Some(b) = cdx.filter(|b| b.starts_with(b"VjCD0100")) {
        put(Kind::Cdx, b);
    }
    if let Some(x) = cdxml.filter(|_| cdxml_type) {
        put(Kind::Cdxml, crate::export::cdxml_document(x).into_bytes());
    }
    // A molfile as length-prefixed lines where ChemDraw writes it so (macOS, Windows); as text on Linux.
    if let Some(m) = molfile.filter(|m| !m.trim().is_empty()) {
        if s == System::Linux {
            put(Kind::Molfile, m.replace("\r\n", "\n").into_bytes());
        } else if let Some(b) = prefixed_lines(m) {
            put(Kind::Molfile, b);
        }
    }
    // SMILES with a NUL, as ChemDraw writes it (macOS, Windows); as text on Linux.
    if let Some(m) = smiles.map(str::trim).filter(|m| !m.is_empty() && m.is_ascii()) {
        put(Kind::Smiles, if s == System::Linux { m.as_bytes().to_vec() } else { [m.as_bytes(), b"\0"].concat() });
    }
    if let Some(j) = octet_json {
        put(Kind::Octet, j.into_bytes());
    }
    Ok(out)
}

// Text alone on the clipboard (Edit > Copy As, a value from a panel), as ChemDraw's Copy As puts it: UTF-8, or on
// Windows UTF-16 with a NUL. Under each of the system's names for text.
fn text_types(s: System, text: &str) -> Vec<(&'static str, Vec<u8>)> {
    let bytes = if s == System::Windows { text.encode_utf16().chain([0]).flat_map(u16::to_le_bytes).collect() } else { text.as_bytes().to_vec() };
    names(s, Kind::Text).iter().map(|n| (*n, bytes.clone())).collect()
}

// The first of a kind's names on the clipboard: its bytes (`read` gives a type's bytes if it is on the clipboard).
fn first(s: System, k: Kind, read: &impl Fn(&str) -> Option<Vec<u8>>) -> Option<Vec<u8>> {
    names(s, k).iter().find_map(|n| read(n))
}

// What paste can use on system `s`, from each type's bytes.
fn clip(s: System, read: impl Fn(&str) -> Option<Vec<u8>>) -> Clip {
    use base64::Engine;
    let mut c = Clip {
        octet: first(s, Kind::Octet, &read).and_then(text_of),
        cdxml: first(s, Kind::Cdxml, &read).and_then(text_of),
        cdx: first(s, Kind::Cdx, &read).map(|b| base64::engine::general_purpose::STANDARD.encode(b)),
        molfile: first(s, Kind::Molfile, &read).and_then(|b| molfile_text(&b)),
        smiles: first(s, Kind::Smiles, &read).and_then(|b| smiles_text(&b)),
        text: first(s, Kind::Text, &read).filter(|b| b.len() <= MAX_TEXT).and_then(|b| plain_text(s, b)),
        picture: first(s, Kind::Png, &read)
            .or_else(|| first(s, Kind::Bitmap, &read).and_then(|b| if s == System::Windows { bmp_file(&b) } else { Some(b) }))
            .map(|b| base64::engine::general_purpose::STANDARD.encode(b)),
        spectrum: None,
    };
    c.spectrum = crate::spectrum::from_clipboard(first(s, Kind::FileUrl, &read).and_then(text_of).as_deref(), c.text.as_deref());
    // The PDF's CDXML when there is no Octet JSON or CDXML type: it holds the whole drawing, arrows, text and shapes
    // too (REL-4, CH-2). ChemDraw's CDX is the page's next choice, for a copy with no PDF.
    if c.octet.is_none() && c.cdxml.is_none() {
        c.cdxml = first(s, Kind::Pdf, &read).and_then(|b| crate::export::cdxml_from_pdf(&b)).or_else(|| first(s, Kind::Emf, &read).and_then(|b| crate::vector::cdxml_from_emf(&b)));
    }
    c
}

// Every name paste may read on system `s`.
#[cfg_attr(target_os = "macos", allow(dead_code))]
fn readable(s: System) -> Vec<&'static str> {
    use Kind::*;
    [Octet, Cdxml, Cdx, Molfile, Smiles, Pdf, Emf, Text, Png, Bitmap, FileUrl].iter().flat_map(|k| names(s, *k).iter().copied()).collect()
}

// A Windows bitmap (CF_DIB: its header, colours and pixels) as a .bmp file, which the page's decoder reads: the
// 14-byte file header goes first, with where the pixels start. None if the header is not sound.
fn bmp_file(dib: &[u8]) -> Option<Vec<u8>> {
    let u32_at = |k: usize| dib.get(k..k + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize);
    let (head, bits, compression, used) = (u32_at(0)?, u16::from_le_bytes([*dib.get(14)?, *dib.get(15)?]), u32_at(16)?, u32_at(32)?);
    let colours = if used > 0 { used } else if bits <= 8 { 1 << bits } else { 0 };
    // A 40-byte header with BI_BITFIELDS (3) has its three colour masks after it.
    let masks = if head == 40 && compression == 3 { 12 } else { 0 };
    let start = head.checked_add(masks)?.checked_add(colours.checked_mul(4)?)?.checked_add(14)?;
    if head < 40 || start > dib.len() + 14 {
        return None;
    }
    let mut out = Vec::with_capacity(dib.len() + 14);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&u32::try_from(dib.len() + 14).ok()?.to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&u32::try_from(start).ok()?.to_le_bytes());
    out.extend_from_slice(dib);
    Some(out)
}

#[cfg(target_os = "macos")]
mod mac {
    use objc2::rc::{autoreleasepool, Retained};
    use objc2::runtime::ProtocolObject;
    use objc2_app_kit::{NSPasteboard, NSPasteboardItem};
    use objc2_foundation::{NSArray, NSData, NSString, NSURL};

    pub fn write(pb: &NSPasteboard, types: &[(&str, Vec<u8>)]) -> Result<(), String> {
        autoreleasepool(|_| {
            let item = NSPasteboardItem::new();
            for (t, bytes) in types {
                if !item.setData_forType(&NSData::with_bytes(bytes), &NSString::from_str(t)) {
                    return Err("The clipboard did not accept the data.".to_string());
                }
            }
            pb.clearContents();
            let items: Retained<NSArray<ProtocolObject<dyn objc2_app_kit::NSPasteboardWriting>>> = NSArray::from_retained_slice(&[ProtocolObject::from_retained(item)]);
            if pb.writeObjects(&items) { Ok(()) } else { Err("The clipboard did not accept the data.".to_string()) }
        })
    }

    // The bytes of each type on the clipboard (64 MB at most), read only if the clipboard has that type.
    pub fn read(pb: &NSPasteboard) -> super::Clip {
        autoreleasepool(|_| {
            let present: Vec<String> = pb.types().map(|a| a.iter().map(|t| t.to_string()).collect()).unwrap_or_default();
            super::clip(super::System::Mac, |t| {
                if !present.iter().any(|p| p == t) {
                    return None;
                }
                let data = pb.dataForType(&NSString::from_str(t))?;
                // A file Finder copied comes as a file reference (file:///.file/id=…): its path instead.
                if t == super::FILE_URL {
                    let u = NSURL::URLWithString(&NSString::from_str(std::str::from_utf8(&data.to_vec()).ok()?))?;
                    return Some(u.filePathURL()?.path()?.to_string().into_bytes());
                }
                (data.len() <= super::MAX_TYPE).then(|| data.to_vec())
            })
        })
    }

    pub fn general() -> Retained<NSPasteboard> {
        NSPasteboard::generalPasteboard()
    }
}

#[cfg(windows)]
#[path = "clipboard_windows.rs"]
mod win;

#[cfg(target_os = "linux")]
#[path = "clipboard_linux.rs"]
mod linux;

// Copy: the selection as a picture, CDX (base64 from the page), CDXML, a molfile, SMILES and Octet's JSON. With
// `text`, only that text (CH-7): the app puts it there, as the webview allows a page to copy only during a click.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn copy_to_clipboard(app: tauri::AppHandle, svg: Option<String>, cdxml: Option<String>, cdxml_type: Option<bool>, cdx: Option<String>, molfile: Option<String>, smiles: Option<String>, octet_json: Option<String>, text: Option<String>) -> Result<(), String> {
    use base64::Engine;
    let cdx = match cdx {
        Some(c) if c.len() <= MAX_TYPE / 3 * 4 + 4 => Some(base64::engine::general_purpose::STANDARD.decode(c).map_err(|_| "Octet cannot copy this.".to_string())?),
        _ => None,
    };
    let types = match text {
        Some(t) if t.len() <= MAX_TYPE => text_types(HERE, &t),
        Some(_) => return Err("This text is too long to copy.".into()),
        None => types(HERE, svg.as_deref(), cdxml.as_deref(), cdxml_type.unwrap_or(false), cdx, molfile.as_deref(), smiles.as_deref(), octet_json)?,
    };
    let _ = &app;
    #[cfg(target_os = "macos")]
    return mac::write(&mac::general(), &types);
    #[cfg(windows)]
    return win::write(&types);
    #[cfg(target_os = "linux")]
    return linux::write(&app, types).await;
    #[allow(unreachable_code)]
    Err("Octet cannot copy on this system.".into())
}

// Paste: everything Octet can use from the clipboard.
#[tauri::command]
pub async fn read_clipboard(app: tauri::AppHandle) -> Result<Clip, String> {
    let _ = &app;
    #[cfg(target_os = "macos")]
    return Ok(mac::read(&mac::general()));
    #[cfg(windows)]
    return win::read(&readable(HERE)).map(|found| clip(HERE, |t| found.iter().find(|(n, _)| *n == t).map(|(_, b)| b.clone())));
    #[cfg(target_os = "linux")]
    return linux::read(&app, readable(HERE)).await.map(|found| clip(HERE, |t| found.iter().find(|(n, _)| *n == t).map(|(_, b)| b.clone())));
    #[allow(unreachable_code)]
    Err("Octet cannot paste on this system.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOL: &str = "\n  Octet\n\n  2  1  0  0  0  0  0  0  0  0999 V2000\n    0.0000    0.0000    0.0000 C   0  0\n    1.0000    0.0000    0.0000 O   0  0\n  1  2  1  0\nM  END\n";

    #[test]
    fn molfile_as_chemdraw_writes_it() {
        let b = prefixed_lines(MOL).unwrap();
        // One length byte per line instead of each line end, so the same length as the text.
        assert_eq!(b.len(), MOL.len());
        assert_eq!(b[0], 0);
        assert_eq!(&b[1..9], b"\x07  Octet");
        assert!(b.ends_with(b"\x06M  END"));
        assert_eq!(molfile_text(&b).as_deref(), Some(MOL));
        // Ordinary text with CR line ends reads too.
        assert_eq!(molfile_text(MOL.replace('\n', "\r").as_bytes()).as_deref(), Some(MOL));
        assert_eq!(prefixed_lines(&"x".repeat(300)), None);
    }

    #[test]
    fn smiles_with_nul() {
        assert_eq!(smiles_text(b"N[C@@H](CC1=CC=CC=C1)C(O)=O\0").as_deref(), Some("N[C@@H](CC1=CC=CC=C1)C(O)=O"));
        assert_eq!(smiles_text(b"CCO").as_deref(), Some("CCO"));
        assert_eq!(smiles_text(b"\0"), None);
    }

    #[test]
    fn copy_types_in_chemdraws_order() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="20pt" height="10pt" viewBox="0 0 20 10"><path d="M1 1H19V2H1Z"/></svg>"#;
        let t = types(System::Mac, Some(svg), Some("<CDXML/>"), true, Some(b"VjCD0100\x04\x03\x02\x01".to_vec()), Some(MOL), Some("CO"), Some("{}".into())).unwrap();
        assert_eq!(t.iter().map(|(t, _)| *t).collect::<Vec<_>>(), [PDF, CDX[0], CDX[1], CDX[2], CDXML[0], CDXML[1], CDXML[2], MOLFILE, SMILES, OCTET]);
        assert!(t[1].1.starts_with(b"VjCD0100") && t[1].1 == t[2].1 && t[1].1 == t[3].1);
        let t = types(System::Mac, Some(svg), Some("<CDXML/>"), true, None, Some(MOL), Some("CO"), Some("{}".into())).unwrap();
        assert_eq!(t.iter().map(|(t, _)| *t).collect::<Vec<_>>(), [PDF, CDXML[0], CDXML[1], CDXML[2], MOLFILE, SMILES, OCTET]);
        assert!(t[1].1.starts_with(b"<?xml") && t[1].1.ends_with(b"<CDXML/>") && t[1].1 == t[2].1 && t[1].1 == t[3].1);
        assert_eq!(t[5].1, b"CO\0");
        // ChemDraw 26.1's Revvity CDXML type pastes too.
        assert!(clip(System::Mac, |n| (n == CDXML[1]).then(|| b"<CDXML/>".to_vec())).cdxml.is_some());
        // Paste reads them all back; the PDF is not needed when Octet's JSON is there.
        let read = |name: &str| t.iter().find(|(n, _)| *n == name).map(|(_, b)| b.clone());
        let c = clip(System::Mac, read);
        assert_eq!((c.octet.as_deref(), c.molfile.as_deref(), c.smiles.as_deref()), (Some("{}"), Some(MOL), Some("CO")));
        // Plain text (a name copied from a paper) comes through for the name field, unless it is too long.
        assert_eq!(clip(System::Mac, |name| (name == TEXT).then(|| b"aspirin".to_vec())).text.as_deref(), Some("aspirin"));
        assert_eq!(clip(System::Mac, |name| (name == TEXT).then(|| vec![b'a'; MAX_TEXT + 1])).text, None);
        let utf16: Vec<u8> = "aspirin".encode_utf16().chain([0]).flat_map(u16::to_le_bytes).collect();
        assert_eq!(clip(System::Windows, |name| (name == UNICODE_TEXT).then(|| utf16.clone())).text.as_deref(), Some("aspirin"));
        // A picture: PNG first, else the system's bitmap (on Windows made a .bmp file).
        assert_eq!(clip(System::Mac, |n| (n == "public.tiff").then(|| b"MM\0*".to_vec())).picture.as_deref(), Some("TU0AKg=="));
        assert_eq!(clip(System::Mac, |n| [Some("public.tiff"), Some("public.png")].contains(&Some(n)).then(|| n.as_bytes().to_vec())).picture.as_deref(), Some("cHVibGljLnBuZw=="));
        let mut dib = vec![0u8; 40 + 4 * 2 + 8];
        dib[0] = 40;
        dib[14] = 1; // 1 bit a pixel: two colours, then the pixels
        let bmp = bmp_file(&dib).unwrap();
        assert_eq!((&bmp[..2], u32::from_le_bytes(bmp[2..6].try_into().unwrap()), u32::from_le_bytes(bmp[10..14].try_into().unwrap())), (&b"BM"[..], 70, 62));
        assert_eq!(&bmp[14..], &dib[..]);
        assert!(clip(System::Windows, |n| (n == DIB).then(|| dib.clone())).picture.is_some());
        assert_eq!(bmp_file(&dib[..20]), None);
        assert_eq!(bmp_file(&[&[40, 0, 0, 0][..], &[0; 28], &[255, 255, 255, 255], &[0; 8]].concat()), None);
        // Without it (a picture copied back out of Word), the CDXML comes out of the PDF.
        let c = clip(System::Mac, |name| (name == PDF).then(|| t[0].1.clone()));
        assert!(c.cdxml.unwrap().contains("<CDXML/>"));
        // The Copy switches in Settings: with all four off, only Octet's JSON goes on; CDXML off keeps it in the PDF.
        let t = types(System::Mac, None, Some("<CDXML/>"), false, None, None, None, Some("{}".into())).unwrap();
        assert_eq!(t.iter().map(|(t, _)| *t).collect::<Vec<_>>(), [OCTET]);
        let t = types(System::Mac, Some(svg), Some("<CDXML/>"), false, None, None, Some("CO"), None).unwrap();
        assert_eq!(t.iter().map(|(t, _)| *t).collect::<Vec<_>>(), [PDF, SMILES]);
        assert!(crate::export::cdxml_from_pdf(&t[0].1).unwrap().contains("<CDXML/>"));
    }

    // REL-4, CH-2: ChemDraw's copy has a CDX as well as its PDF. Paste still takes the whole drawing from the PDF.
    #[test]
    fn chemdraws_copy_pastes_from_its_pdf() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="20pt" height="10pt" viewBox="0 0 20 10"><path d="M1 1H19V2H1Z"/></svg>"#;
        let pdf = crate::export::make_pdf(svg, Some("<CDXML><page><arrow/></page></CDXML>")).unwrap();
        for cdx in CDX {
            let c = clip(System::Mac, |name| match name {
                PDF => Some(pdf.clone()),
                n if n == cdx => Some(b"VjCD0100".to_vec()),
                MOLFILE => Some(prefixed_lines(MOL).unwrap()),
                _ => None,
            });
            assert!(c.cdxml.as_deref().is_some_and(|x| x.contains("<arrow/>")), "{cdx}");
            assert!(c.cdx.is_some() && c.molfile.is_some());
        }
        // Sam's dump of ChemDraw 22.2's copy (see export.rs reads_chemdraws_pdf).
        let Ok(dir) = std::env::var("OCTET_CLIPBOARD_DUMPS") else { return };
        let c = clip(System::Mac, |name| std::fs::read(format!("{dir}/chemdraw-copy/0_{name}")).ok());
        assert!(c.cdxml.unwrap().contains("<CDXML") && c.cdx.is_some() && c.molfile.is_some() && c.smiles.is_some());
    }

    // REL-3, CH-1: Windows and Linux copy the same things as the Mac, under their own names: on Windows a vector EMF and
    // a bitmap for the picture, on Linux a PNG and an SVG. Paste reads them back.
    #[test]
    fn windows_and_linux_copy_the_same() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="20pt" height="10pt" viewBox="0 0 20 10"><path d="M1 1H19V2H1Z"/></svg>"#;
        let cdx = b"VjCD0100\x04\x03\x02\x01".to_vec();
        let all = |s| types(s, Some(svg), Some("<CDXML/>"), true, Some(cdx.clone()), Some(MOL), Some("CO"), Some("{}".into())).unwrap();
        let t = all(System::Windows);
        assert_eq!(t.iter().map(|(t, _)| *t).collect::<Vec<_>>(), [ENHMETAFILE, DIB, "ChemDraw Interchange Format", "CDXML", "MDLCT", "SMILES", "application/x-octet+json"]);
        assert_eq!(&t[0].1[40..44], b" EMF");
        // A picture copied back out of Word keeps its EMF: with nothing else, paste reads the drawing from it.
        let c = clip(System::Windows, |n| (n == ENHMETAFILE).then(|| t[0].1.clone()));
        assert_eq!(c.cdxml.as_deref(), Some("<CDXML/>"));
        // 20 pt by 10 pt at 600 dpi: 167 by 84 pixels, as a bottom-up 24-bit bitmap on white with rows of 504 bytes.
        let dib = &t[1].1;
        let int = |k: usize| i32::from_le_bytes(dib[k..k + 4].try_into().unwrap());
        assert_eq!((int(0), int(4), int(8), dib[14]), (40, 167, 84, 24));
        assert_eq!(dib.len(), 40 + 504 * 84);
        assert_eq!(&dib[40..43], [255, 255, 255], "the bottom row is white");
        assert!(dib[40..].chunks(3).any(|c| c == [0, 0, 0]), "the bar is black");
        assert_eq!((t[4].1.clone(), t[5].1.clone()), (prefixed_lines(MOL).unwrap(), b"CO\0".to_vec()));
        let read = |t: &Vec<(&str, Vec<u8>)>, n: &str| t.iter().find(|(x, _)| *x == n).map(|(_, b)| b.clone());
        let c = clip(System::Windows, |n| read(&t, n));
        assert_eq!((c.octet.as_deref(), c.molfile.as_deref(), c.smiles.as_deref()), (Some("{}"), Some(MOL), Some("CO")));
        assert!(c.cdxml.unwrap().ends_with("<CDXML/>") && c.cdx.is_some());
        // A block of Windows memory can be larger than what was put in it: text reads without the NULs after it.
        let c = clip(System::Windows, |n| (n == "application/x-octet+json").then(|| b"{}\0\0\0".to_vec()));
        assert_eq!(c.octet.as_deref(), Some("{}"));
        let t = all(System::Linux);
        assert_eq!(t.iter().map(|(t, _)| *t).collect::<Vec<_>>(), ["image/png", "image/svg+xml", "chemical/x-cdx", "chemical/x-cdxml", "chemical/x-mdl-molfile", "chemical/x-daylight-smiles", "application/x-octet+json"]);
        assert_eq!((t[1].1.as_slice(), t[4].1.as_slice(), t[5].1.as_slice()), (svg.as_bytes(), MOL.as_bytes(), b"CO".as_slice()));
        let c = clip(System::Linux, |n| read(&t, n));
        assert_eq!((c.octet.as_deref(), c.molfile.as_deref(), c.smiles.as_deref()), (Some("{}"), Some(MOL), Some("CO")));
        assert!(c.cdxml.unwrap().ends_with("<CDXML/>") && c.cdx.is_some());
        // The Copy switches: with all five off, only Octet's JSON goes on.
        for s in [System::Windows, System::Linux] {
            let t = types(s, None, None, false, None, None, None, Some("{}".into())).unwrap();
            assert_eq!(t.len(), 1);
        }
        // Paste reads only names its system has.
        assert!(readable(System::Windows).contains(&"MDLCT") && !readable(System::Linux).contains(&PDF));
    }

    // CH-7: Copy As puts the text alone, under each system's names for text (on Windows UTF-16 with a NUL).
    #[test]
    fn copy_as_puts_text() {
        assert_eq!(text_types(System::Mac, "CCO"), [("public.utf8-plain-text", b"CCO".to_vec())]);
        assert_eq!(text_types(System::Windows, "Cé"), [(UNICODE_TEXT, vec![b'C', 0, 0xe9, 0, 0, 0])]);
        assert_eq!(text_types(System::Linux, "CCO").iter().map(|(n, _)| *n).collect::<Vec<_>>(), ["text/plain;charset=utf-8", "UTF8_STRING", "text/plain"]);
    }

    // A long drawing goes on the clipboard at a lower resolution, so the picture has at most 8,192 pixels on a side.
    #[test]
    fn large_pictures_fit() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="3000pt" height="100pt" viewBox="0 0 3000 100"><path d="M1 1H2999V99H1Z"/></svg>"#;
        let (png, dib) = crate::export::clipboard_pictures(svg, true).unwrap();
        let width = u32::from_be_bytes(png[16..20].try_into().unwrap());
        assert_eq!(width, 8192);
        assert_eq!(i32::from_le_bytes(dib.unwrap()[4..8].try_into().unwrap()), 8192);
    }

    // On a private pasteboard, so the user's clipboard is untouched.
    #[cfg(target_os = "macos")]
    #[test]
    fn pasteboard_keeps_type_order() {
        use objc2_app_kit::NSPasteboard;
        let pb = NSPasteboard::pasteboardWithUniqueName();
        let t = vec![(PDF, b"%PDF-1.7".to_vec()), (MOLFILE, prefixed_lines(MOL).unwrap()), (SMILES, b"CO\0".to_vec()), (OCTET, b"{}".to_vec())];
        mac::write(&pb, &t).unwrap();
        let names: Vec<String> = pb.types().unwrap().iter().map(|s| s.to_string()).collect();
        let ours: Vec<&str> = names.iter().map(String::as_str).filter(|n| t.iter().any(|(x, _)| x == n)).collect();
        assert_eq!(ours, [PDF, MOLFILE, SMILES, OCTET], "{names:?}");
        let c = mac::read(&pb);
        assert_eq!((c.octet.as_deref(), c.smiles.as_deref(), c.molfile.as_deref(), c.text.as_deref()), (Some("{}"), Some("CO"), Some(MOL), None));
        pb.clearContents();
    }
}
