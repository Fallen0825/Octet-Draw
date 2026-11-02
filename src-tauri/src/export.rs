// File > Export… and the picture on the clipboard. The page sends a standalone SVG of the drawing at true size
// (src/export.ts); this makes a vector PDF, EPS or EMF (vector.rs), or a PNG, TIFF, JPEG or GIF of it. The PDF carries the drawing's CDXML in its catalog
// /Metadata stream, as ChemDraw's does, so a picture pasted into Word and copied out again pastes back into
// Octet and ChemDraw as a structure. cdxml_from_pdf reads it back from any PDF, treated as hostile.

use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use base64::Engine;
use pdf_writer::{Content, Filter, Finish, Name, Pdf, Rect, Ref};
use svg2pdf::usvg::{self, fontdb};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

const MAX_SVG: usize = 64 << 20;
const MAX_PDF: usize = 64 << 20;
const MAX_XML: usize = 32 << 20;

// The system's fonts, loaded once. Labels are embedded from them as subsets (Arial on the Mac and Windows).
// Linux has no Arial unless the user adds it, so labels fall back to a font with Arial's widths (Liberation Sans,
// Arimo), or else a common sans; with no fallback the labels would be left out of the PDF.
const NO_ARIAL: [&str; 5] = ["Liberation Sans", "Arimo", "Helvetica", "DejaVu Sans", "Noto Sans"];
fn fonts() -> Arc<fontdb::Database> {
    static FONTS: OnceLock<Arc<fontdb::Database>> = OnceLock::new();
    FONTS.get_or_init(|| {
        let mut db = fontdb::Database::new();
        db.load_system_fonts();
        let has = |db: &fontdb::Database, name: &str| db.faces().any(|f| f.families.iter().any(|(n, _)| n == name));
        if !has(&db, "Arial") {
            if let Some(name) = NO_ARIAL.into_iter().find(|n| has(&db, n)) {
                db.set_sans_serif_family(name);
            }
        }
        Arc::new(db)
    }).clone()
}

// Every font family on the computer, for the font lists, as ChemDraw's Font menu lists every installed font. Hidden
// system fonts (their names start with a dot) are left out.
#[tauri::command]
pub async fn system_fonts() -> Vec<String> {
    let mut names: Vec<String> = fonts()
        .faces()
        .filter_map(|f| f.families.first().map(|(n, _)| n.trim().to_string()))
        .filter(|n| !n.is_empty() && !n.starts_with('.') && n.len() <= 64)
        .collect();
    names.sort_by_cached_key(|n| n.to_lowercase());
    names.dedup();
    names
}

pub fn tree(svg: &str) -> Result<usvg::Tree, String> {
    if svg.len() > MAX_SVG {
        return Err("This drawing is too large to export.".into());
    }
    let mut opt = usvg::Options::default();
    opt.fontdb = fonts();
    // usvg reads <image> files from disk by default. The picture holds only what the page drew.
    opt.image_href_resolver.resolve_string = Box::new(|_, _| None);
    usvg::Tree::from_str(svg, &opt).map_err(|e| {
        eprintln!("usvg: {e}");
        "Octet cannot draw this picture.".to_string()
    })
}

// The CDXML as ChemDraw hides it: the XML declaration at byte 0 and the CDXML DOCTYPE on line 2.
pub fn cdxml_document(cdxml: &str) -> String {
    let mut s = cdxml.trim_start_matches('\u{feff}').trim_start();
    if s.starts_with("<?xml") {
        s = s.find("?>").map_or(s, |k| &s[k + 2..]).trim_start();
    }
    if s.starts_with("<!DOCTYPE") {
        s = s.find('>').map_or(s, |k| &s[k + 1..]).trim_start();
    }
    format!("<?xml version=\"1.0\" encoding=\"UTF-8\" ?>\n<!DOCTYPE CDXML SYSTEM \"http://www.cambridgesoft.com/xml/cdxml.dtd\" >\n{s}")
}

// A vector PDF of the SVG at true size: the page is the SVG's box (1 pt per pt, CSS units honoured), text stays
// text in embedded font subsets, and nothing paints the background. With CDXML, the catalog gets a /Metadata
// stream holding it (/Type /Metadata /Subtype /XML, Flate, /Vendor /org.octet).
pub fn make_pdf(svg: &str, cdxml: Option<&str>) -> Result<Vec<u8>, String> {
    let tree = tree(svg)?;
    let (chunk, picture) = svg2pdf::to_chunk(&tree, svg2pdf::ConversionOptions::default()).map_err(|e| e.to_string())?;
    let mut alloc = Ref::new(1);
    let [catalog, pages, page, content, meta] = [(); 5].map(|_| alloc.bump());
    let mut map = HashMap::new();
    let chunk = chunk.renumber(|old| *map.entry(old).or_insert_with(|| alloc.bump()));
    // usvg measures in CSS px (96 to the inch); PDF in points (72). Rounded, so 86 pt stays 86, not 85.99999.
    let pt = |px: f32| (px * 750.0).round() / 1000.0;
    let (w, h) = (pt(tree.size().width()), pt(tree.size().height()));

    let mut pdf = Pdf::new();
    let mut cat = pdf.catalog(catalog);
    cat.pages(pages);
    if cdxml.is_some() {
        cat.metadata(meta);
    }
    cat.finish();
    pdf.pages(pages).kids([page]).count(1);
    let mut p = pdf.page(page);
    p.media_box(Rect::new(0.0, 0.0, w, h)).parent(pages).contents(content);
    p.resources().x_objects().pair(Name(b"S"), map[&picture]);
    p.finish();
    let mut c = Content::new();
    c.save_state().transform([w, 0.0, 0.0, h, 0.0, 0.0]).x_object(Name(b"S")).restore_state();
    pdf.stream(content, &c.finish());
    pdf.extend(&chunk);
    if let Some(x) = cdxml {
        let data = miniz_oxide::deflate::compress_to_vec_zlib(cdxml_document(x).as_bytes(), 6);
        pdf.metadata(meta, &data).filter(Filter::FlateDecode).pair(Name(b"Vendor"), Name(b"org.octet"));
    }
    Ok(pdf.finish())
}

// The SVG drawn at `dpi` of its true size, white or transparent behind.
fn draw(tree: &usvg::Tree, dpi: f32, transparent: bool) -> Result<resvg::tiny_skia::Pixmap, String> {
    use resvg::tiny_skia::{Color, Pixmap, Transform};
    let scale = dpi / 96.0;
    let (w, h) = ((tree.size().width() * scale).ceil() as u32, (tree.size().height() * scale).ceil() as u32);
    let too_big = "This picture is too large at this resolution. Choose a lower resolution.";
    if w > 20_000 || h > 20_000 || u64::from(w) * u64::from(h) > 100_000_000 {
        return Err(too_big.into());
    }
    let mut pixmap = Pixmap::new(w.max(1), h.max(1)).ok_or(too_big)?;
    if !transparent {
        pixmap.fill(Color::WHITE);
    }
    resvg::render(tree, Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    Ok(pixmap)
}

// A drawn picture as width, height and RGBA rows (not premultiplied).
fn rgba_of(pixmap: &resvg::tiny_skia::Pixmap) -> (u32, u32, Vec<u8>) {
    let rgba = pixmap.pixels().iter().flat_map(|p| {
        let c = p.demultiply();
        [c.red(), c.green(), c.blue(), c.alpha()]
    }).collect();
    (pixmap.width(), pixmap.height(), rgba)
}

// The SVG as pixels at `dpi` (of its true size), white or transparent behind.
fn pixels(svg: &str, dpi: f32, transparent: bool) -> Result<(u32, u32, Vec<u8>), String> {
    Ok(rgba_of(&draw(&tree(svg)?, dpi, transparent)?))
}

fn dpi_of(dpi: f32) -> f32 {
    if dpi.is_finite() { dpi.clamp(36.0, 2400.0) } else { 300.0 }
}

// A PNG of the SVG at `dpi` (of its true size), white or transparent behind. It records its resolution, so
// Word, PowerPoint and Keynote place it at true size.
pub fn make_png(svg: &str, dpi: f32, transparent: bool) -> Result<Vec<u8>, String> {
    let dpi = dpi_of(dpi);
    png_of(&draw(&tree(svg)?, dpi, transparent)?, dpi)
}

fn png_of(pixmap: &resvg::tiny_skia::Pixmap, dpi: f32) -> Result<Vec<u8>, String> {
    let (w, h, rgba) = rgba_of(pixmap);
    let mut out = Vec::new();
    let mut enc = png::Encoder::new(&mut out, w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let ppm = (dpi / 0.0254).round() as u32;
    enc.set_pixel_dims(Some(png::PixelDimensions { xppu: ppm, yppu: ppm, unit: png::Unit::Meter }));
    let mut writer = enc.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(&rgba).map_err(|e| e.to_string())?;
    writer.finish().map_err(|e| e.to_string())?;
    Ok(out)
}

// The picture on the clipboard on Windows and Linux, where Office pastes no PDF: a transparent PNG at 600 dpi of
// true size (lower for a large drawing, so it has at most 16 million pixels and 8,192 on a side), and with `bitmap`
// the same on white as a Windows bitmap (a DIB), for programs that read no PNG.
pub fn clipboard_pictures(svg: &str, bitmap: bool) -> Result<(Vec<u8>, Option<Vec<u8>>), String> {
    clipboard_pixels(&tree(svg)?, bitmap)
}

pub fn clipboard_pixels(tree: &usvg::Tree, bitmap: bool) -> Result<(Vec<u8>, Option<Vec<u8>>), String> {
    let (w, h) = (tree.size().width().max(1.0), tree.size().height().max(1.0));
    let dpi = 600f32.min(96.0 * (16e6 / (w * h)).sqrt()).min(96.0 * 8192.0 / w.max(h));
    let pixmap = draw(tree, dpi, true)?;
    Ok((png_of(&pixmap, dpi)?, bitmap.then(|| dib(&pixmap, dpi))))
}

// A device-independent bitmap as the Windows clipboard holds it (CF_DIB): a BITMAPINFOHEADER, then rows of blue,
// green and red from the bottom up, each padded to 4 bytes. The picture is put on white.
fn dib(pixmap: &resvg::tiny_skia::Pixmap, dpi: f32) -> Vec<u8> {
    let (w, h) = (pixmap.width() as usize, pixmap.height() as usize);
    let row = (w * 3 + 3) & !3;
    let ppm = ((dpi / 0.0254).round() as i32).to_le_bytes();
    let mut out = Vec::with_capacity(40 + row * h);
    out.extend(40u32.to_le_bytes());
    out.extend((w as i32).to_le_bytes());
    out.extend((h as i32).to_le_bytes());
    out.extend(1u16.to_le_bytes()); // planes
    out.extend(24u16.to_le_bytes()); // bits per pixel
    out.extend(0u32.to_le_bytes()); // BI_RGB
    out.extend(((row * h) as u32).to_le_bytes());
    out.extend(ppm);
    out.extend(ppm);
    out.extend([0u8; 8]); // colours used, colours important
    let px = pixmap.pixels();
    for y in (0..h).rev() {
        let start = out.len();
        // Premultiplied colour on white: each channel plus what the alpha leaves.
        for c in &px[y * w..(y + 1) * w] {
            let white = 255 - c.alpha();
            out.extend([c.blue().saturating_add(white), c.green().saturating_add(white), c.red().saturating_add(white)]);
        }
        out.resize(start + row, 0);
    }
    out
}

// A JPEG of the SVG at `dpi`, on white (JPEG has no transparency), at quality 95, with its resolution.
pub fn make_jpeg(svg: &str, dpi: f32) -> Result<Vec<u8>, String> {
    use image::codecs::jpeg::{JpegEncoder, PixelDensity};
    let dpi = dpi_of(dpi);
    let (w, h, rgba) = pixels(svg, dpi, false)?;
    let rgb: Vec<u8> = rgba.chunks_exact(4).flat_map(|p| [p[0], p[1], p[2]]).collect();
    let mut out = Vec::new();
    let mut enc = JpegEncoder::new_with_quality(&mut out, 95);
    enc.set_pixel_density(PixelDensity::dpi(dpi.round() as u16));
    enc.encode(&rgb, w, h, image::ExtendedColorType::Rgb8).map_err(|e| e.to_string())?;
    Ok(out)
}

// A GIF of the SVG at `dpi`, white or transparent behind: 256 colours at most, chosen for the picture. GIF records no
// resolution.
pub fn make_gif(svg: &str, dpi: f32, transparent: bool) -> Result<Vec<u8>, String> {
    let (w, h, mut rgba) = pixels(svg, dpi_of(dpi), transparent)?;
    if w > u32::from(u16::MAX) || h > u32::from(u16::MAX) {
        return Err("This picture is too large for a GIF. Choose a lower resolution.".into());
    }
    // GIF transparency is on or off: a pixel under half covered is clear, the rest are solid.
    for p in rgba.chunks_exact_mut(4) {
        p[3] = if p[3] < 128 { 0 } else { 255 };
    }
    let mut out = Vec::new();
    let mut enc = image::codecs::gif::GifEncoder::new_with_speed(&mut out, 10);
    enc.encode(&rgba, w, h, image::ExtendedColorType::Rgba8).map_err(|e| e.to_string())?;
    drop(enc);
    Ok(out)
}

// A TIFF of the SVG at `dpi`, white or transparent behind: baseline RGB (RGBA with transparency), 8 bits a sample,
// PackBits compression, its resolution in pixels per inch. Little-endian, a strip every 16 rows.
pub fn make_tiff(svg: &str, dpi: f32, transparent: bool) -> Result<Vec<u8>, String> {
    let dpi = dpi_of(dpi);
    let (w, h, rgba) = pixels(svg, dpi, transparent)?;
    let spp: u32 = if transparent { 4 } else { 3 };
    let rows = 16u32;
    let mut out = vec![b'I', b'I', 42, 0, 0, 0, 0, 0];
    let (mut offsets, mut counts) = (Vec::new(), Vec::new());
    for strip in rgba.chunks((w * 4 * rows) as usize) {
        offsets.push(out.len() as u32);
        let start = out.len();
        for row in strip.chunks_exact((w * 4) as usize) {
            let line: Vec<u8> = row.chunks_exact(4).flat_map(|p| p[..spp as usize].to_vec()).collect();
            packbits(&line, &mut out);
        }
        counts.push((out.len() - start) as u32);
        if out.len() % 2 == 1 {
            out.push(0);
        }
    }
    // The arrays the directory points to: bits per sample, strip offsets and byte counts, the two resolutions.
    let at = |data: &[u8], out: &mut Vec<u8>| -> u32 {
        let o = out.len() as u32;
        out.extend_from_slice(data);
        if out.len() % 2 == 1 {
            out.push(0);
        }
        o
    };
    let le = |v: &[u32]| v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>();
    let bits = at(&vec![8u8, 0].repeat(spp as usize), &mut out);
    let offs = at(&le(&offsets), &mut out);
    let lens = at(&le(&counts), &mut out);
    let res = at(&le(&[(dpi * 100.0).round() as u32, 100]), &mut out);
    let n = offsets.len() as u32;
    // Tags in order: (tag, type, count, value or offset). Types: 3 SHORT, 4 LONG, 5 RATIONAL.
    let mut tags: Vec<(u16, u16, u32, u32)> = vec![
        (256, 4, 1, w), (257, 4, 1, h), (258, 3, spp, bits), (259, 3, 1, 32773), (262, 3, 1, 2),
        (273, 4, n, if n == 1 { offsets[0] } else { offs }), (277, 3, 1, spp), (278, 4, 1, rows),
        (279, 4, n, if n == 1 { counts[0] } else { lens }), (282, 5, 1, res), (283, 5, 1, res), (284, 3, 1, 1), (296, 3, 1, 2),
    ];
    if transparent {
        tags.push((338, 3, 1, 2)); // ExtraSamples: unassociated alpha
    }
    let dir = out.len() as u32;
    out[4..8].copy_from_slice(&dir.to_le_bytes());
    out.extend_from_slice(&(tags.len() as u16).to_le_bytes());
    for (tag, kind, count, value) in tags {
        out.extend_from_slice(&tag.to_le_bytes());
        out.extend_from_slice(&kind.to_le_bytes());
        out.extend_from_slice(&count.to_le_bytes());
        // A single SHORT sits in the first two bytes of the value.
        out.extend_from_slice(&value.to_le_bytes());
    }
    out.extend_from_slice(&0u32.to_le_bytes());
    Ok(out)
}
// PackBits (TIFF compression 32773): runs of 2 to 128 equal bytes as (1 - n, byte), the rest as (n - 1, n bytes).
fn packbits(line: &[u8], out: &mut Vec<u8>) {
    let mut i = 0;
    while i < line.len() {
        let mut run = 1;
        while i + run < line.len() && run < 128 && line[i + run] == line[i] {
            run += 1;
        }
        if run >= 2 {
            out.push((1 - run as i32) as i8 as u8);
            out.push(line[i]);
            i += run;
            continue;
        }
        let start = i;
        while i < line.len() && i - start < 128 && !(i + 1 < line.len() && line[i + 1] == line[i]) {
            i += 1;
        }
        if i == start {
            i += 1;
        }
        out.push((i - start - 1) as u8);
        out.extend_from_slice(&line[start..i]);
    }
}

// What File > Export… can write: id, name in the Save dialog, extensions (the first is the default). The MDL formats
// are V3000, and V2000 apart, with ChemDraw's names. The pictures are ChemDraw's Save As pictures. A template set saved
// as a ChemDraw template file (src/ctp.ts) is a template document (CTP) or stationery (CTS), with ChemDraw's names: the
// same CDX file.
const FORMATS: [(&str, &str, &[&str]); 23] = [
    ("oct", "Octet drawing", &["oct"]),
    ("pdf", "PDF", &["pdf"]),
    ("svg", "SVG", &["svg"]),
    ("png", "PNG", &["png"]),
    ("emf", "Windows Enhanced Metafile (EMF)", &["emf"]),
    ("eps", "Encapsulated PostScript (EPS)", &["eps"]),
    ("tif", "TIFF", &["tif", "tiff"]),
    ("jpg", "JPEG", &["jpg", "jpeg"]),
    ("gif", "GIF", &["gif"]),
    ("cdxml", "ChemDraw XML (CDXML)", &["cdxml"]),
    ("cdx", "ChemDraw (CDX)", &["cdx"]),
    ("mol", "MDL Molfile", &["mol"]),
    ("mol2000", "MDL Molfile V2000", &["mol"]),
    ("sdf", "MDL SDfile", &["sdf", "sd"]),
    ("sdf2000", "MDL SDfile V2000", &["sdf", "sd"]),
    ("rxn", "Reaction Molfile (RXN)", &["rxn"]),
    ("rxn2000", "Reaction Molfile V2000 (RXN)", &["rxn"]),
    ("rdf", "MDL RDfile", &["rdf", "rd"]),
    ("rdf2000", "MDL RDfile V2000", &["rdf", "rd"]),
    ("smi", "SMILES", &["smi", "smiles"]),
    ("inchi", "InChI", &["inchi", "txt"]),
    ("ctp", "ChemDraw Template (CTP)", &["ctp"]),
    ("cts", "ChemDraw Template Style Sheet (CTS)", &["cts"]),
];

#[derive(serde::Deserialize)]
pub struct Export {
    name: String,
    format: Option<String>,
    oct: Option<String>,
    svg: Option<String>,
    cdxml: Option<String>,
    cdx: Option<String>, // base64
    ctp: Option<String>, // a template set's ChemDraw template file, base64
    mdl: Option<String>, // the text of the MDL format asked for
    smiles: Option<String>,
    inchi: Option<String>,
    dpi: Option<f32>,
    transparent: Option<bool>,
}

// Files exported this session. The page can ask to show one of them in the file manager (reveal_file), never
// another file.
#[derive(Default)]
pub struct Exported(pub Mutex<HashSet<PathBuf>>);

// File > Export…: asks where to save, then writes the format the file name's extension asks for. The page sends
// the drawing in every form it has; the Save dialog offers those (or only `format`, if given). The page never
// names a path, so Octet writes only where the user chose. Returns the path, or None if cancelled.
#[tauri::command]
pub async fn export_file(app: AppHandle, exported: State<'_, Exported>, export: Export) -> Result<Option<String>, String> {
    let e = export;
    let has = |id: &str| match id {
        "oct" => e.oct.is_some(),
        "pdf" | "svg" | "png" | "emf" | "eps" | "tif" | "jpg" | "gif" => e.svg.is_some(),
        "cdxml" => e.cdxml.is_some(),
        "cdx" => e.cdx.is_some(),
        "inchi" => e.inchi.is_some(),
        "smi" => e.smiles.is_some(),
        "ctp" | "cts" => e.ctp.is_some(),
        _ => e.mdl.is_some(),
    };
    let offered: Vec<_> = FORMATS.iter().filter(|f| has(f.0) && e.format.as_deref().is_none_or(|x| x == f.0)).collect();
    let Some(first) = offered.first() else {
        return Err("There is nothing to export in this format.".into());
    };
    let stem: String = e.name.trim_end_matches(".octet").chars().filter(|c| !c.is_control() && !matches!(c, '/' | '\\' | ':')).take(100).collect();
    let stem = if stem.trim().is_empty() { "Untitled".into() } else { stem };
    let mut dialog = app.dialog().file().set_file_name(format!("{stem}.{}", first.2[0]));
    for f in &offered {
        dialog = dialog.add_filter(f.1, f.2);
    }
    let Some(file) = dialog.blocking_save_file() else {
        return Ok(None);
    };
    let mut path: PathBuf = file.into_path().map_err(|e| e.to_string())?;
    let ext = path.extension().and_then(|x| x.to_str()).map(str::to_ascii_lowercase);
    let format = match offered.iter().find(|f| ext.as_deref().is_some_and(|x| f.2.contains(&x))) {
        Some(f) => f.0,
        None => {
            path.as_mut_os_string().push(format!(".{}", first.2[0]));
            first.0
        }
    };
    let text = |s: &Option<String>| s.clone().unwrap_or_default().into_bytes();
    let bytes = match format {
        "pdf" => make_pdf(e.svg.as_deref().unwrap_or_default(), e.cdxml.as_deref())?,
        "svg" => format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{}\n", e.svg.as_deref().unwrap_or_default()).into_bytes(),
        "png" => make_png(e.svg.as_deref().unwrap_or_default(), e.dpi.unwrap_or(300.0), e.transparent.unwrap_or(false))?,
        "tif" => make_tiff(e.svg.as_deref().unwrap_or_default(), e.dpi.unwrap_or(300.0), e.transparent.unwrap_or(false))?,
        "jpg" => make_jpeg(e.svg.as_deref().unwrap_or_default(), e.dpi.unwrap_or(300.0))?,
        "gif" => make_gif(e.svg.as_deref().unwrap_or_default(), e.dpi.unwrap_or(300.0), e.transparent.unwrap_or(false))?,
        "emf" => crate::vector::make_emf(&tree(e.svg.as_deref().unwrap_or_default())?, None),
        "eps" => crate::vector::make_eps(&tree(e.svg.as_deref().unwrap_or_default())?),
        "oct" => text(&e.oct),
        "cdxml" => text(&e.cdxml),
        "cdx" => base64::engine::general_purpose::STANDARD.decode(e.cdx.as_deref().unwrap_or_default()).map_err(|_| "Octet cannot write this drawing as CDX.".to_string())?,
        "ctp" | "cts" => base64::engine::general_purpose::STANDARD.decode(e.ctp.as_deref().unwrap_or_default()).map_err(|_| "Octet cannot write this template file.".to_string())?,
        "inchi" => format!("{}\n", e.inchi.as_deref().unwrap_or_default().trim()).into_bytes(),
        "smi" => format!("{}\n", e.smiles.as_deref().unwrap_or_default().trim()).into_bytes(),
        _ => text(&e.mdl),
    };
    crate::write_atomic(&path, &bytes).map_err(|e| e.to_string())?;
    exported.0.lock().unwrap().insert(path.clone());
    Ok(Some(path.to_string_lossy().into_owned()))
}

// ---- Reading CDXML back out of a PDF: Octet's, ChemDraw's, or either after Word ----
// The PDF is hostile: every index is checked, nothing recurses, a dictionary is read from at most 16 KB,
// at most 10,000 objects are looked at, and inflating stops at 32 MB. The searches for a stream's end and for an
// indirect /Length read at most four times the PDF's size in all, and the check for "endstream" where /Length says
// reads 256 bytes (REL-25): before, a crafted 63 MB PDF with thousands of streams kept a core busy for minutes.

// The drawing's CDXML in a PDF: the catalog's /Metadata, then any /Type /Metadata /Subtype /XML stream, then
// Octet's first test form (base64 CDXML in the document info /Keywords). Clipboard spec 4.3 and 4.4.
pub fn cdxml_from_pdf(pdf: &[u8]) -> Option<String> {
    if pdf.len() > MAX_PDF || !pdf.starts_with(b"%PDF") {
        return None;
    }
    let catalog = rfind(pdf, b"/Root").and_then(|i| reference(pdf, i + 5)).and_then(|(n, _)| object(pdf, n));
    let meta = catalog.and_then(|at| dict(pdf, at)).and_then(|(d, _)| match get(&d, b"Metadata") {
        Some(Val::Ref(n)) => object(pdf, *n),
        _ => None,
    });
    let budget = Cell::new(pdf.len().saturating_mul(4));
    if let Some(s) = meta.and_then(|at| xml_stream(pdf, at, false, &budget)) {
        return Some(s);
    }
    let headers = (0..pdf.len().saturating_sub(2)).filter(|&i| &pdf[i..i + 3] == b"obj" && obj_header(pdf, i));
    if let Some(s) = headers.take(10_000).find_map(|i| xml_stream(pdf, i + 3, true, &budget)) {
        return Some(s);
    }
    let i = find(pdf, b"octet-cdxml-base64:")? + 19;
    let end = pdf[i..].iter().position(|c| !(c.is_ascii_alphanumeric() || b"+/=".contains(c))).map_or(pdf.len(), |k| i + k);
    let raw = base64::engine::general_purpose::STANDARD.decode(&pdf[i..end]).ok()?;
    is_cdxml(&raw).then(|| String::from_utf8_lossy(&raw).into_owned())
}

// CDXML starts with the XML declaration (after an optional byte order mark) and names CDXML near the top.
// Laxer than ChemDraw (which wants the DOCTYPE in the first 256 bytes); the CDXML is read as hostile anyway.
fn is_cdxml(head: &[u8]) -> bool {
    let head = &head[..head.len().min(1024)];
    let s = head.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(head);
    let s = &s[s.iter().position(|c| !c.is_ascii_whitespace()).unwrap_or(s.len())..];
    s.starts_with(b"<?xml") && (find(s, b"<!DOCTYPE CDXML").is_some() || find(s, b"<CDXML").is_some())
}

// The stream of the object whose dictionary starts at `at`, decoded, if it is CDXML. With `marked`, only a
// stream whose dictionary says /Type /Metadata (or /MetaData) and /Subtype /XML. `budget`: the bytes that the
// searches here may still read; each search takes what it reads from it.
fn xml_stream(pdf: &[u8], at: usize, marked: bool, budget: &Cell<usize>) -> Option<String> {
    let (d, end) = dict(pdf, at)?;
    let name = |k: &[u8]| match get(&d, k) {
        Some(Val::Name(n)) => Some(*n),
        _ => None,
    };
    if marked && !(matches!(name(b"Type"), Some(b"Metadata" | b"MetaData")) && name(b"Subtype") == Some(b"XML")) {
        return None;
    }
    let flate = match get(&d, b"Filter") {
        None => false,
        Some(Val::Name(b"FlateDecode")) => true,
        Some(Val::Array(a)) if names(a) == [b"FlateDecode".as_slice()] => true,
        Some(Val::Array(a)) if names(a).is_empty() => false,
        _ => return None,
    };
    // The data starts after "stream" and its line end. /Length says where it ends, if it agrees with the
    // "endstream" there; otherwise the first "endstream" does. The keywords are looked for in the 256 bytes after
    // where they should be, as every /Length can point at one long run.
    let near = |e: usize| &pdf[..pdf.len().min(e.saturating_add(256))];
    let mut i = skip_white(near(end), end);
    if word(near(i), i) != b"stream" {
        return None;
    }
    i += 6;
    i += usize::from(pdf.get(i) == Some(&b'\r'));
    i += usize::from(pdf.get(i) == Some(&b'\n'));
    let spend = |n: usize| budget.get().checked_sub(n).map(|left| budget.set(left)).is_some();
    let length = match get(&d, b"Length") {
        Some(Val::Int(n)) => Some(*n),
        // object() reads the whole PDF.
        Some(Val::Ref(r)) if spend(pdf.len()) => object(pdf, *r).and_then(|j| int(pdf, skip_white(pdf, j))).map(|(n, _)| n),
        _ => None,
    };
    let stated = length.and_then(|n| i.checked_add(usize::try_from(n).ok()?)).filter(|&e| e <= pdf.len() && word(near(e), skip_white(near(e), e)) == b"endstream");
    let end = stated.or_else(|| {
        let rest = pdf.get(i..)?;
        let window = &rest[..rest.len().min(budget.get())];
        let found = find(window, b"endstream");
        spend(found.map_or(window.len(), |k| k + 9));
        let mut e = i + found?;
        e -= usize::from(e > i && pdf[e - 1] == b'\n');
        e -= usize::from(e > i && pdf[e - 1] == b'\r');
        Some(e)
    })?;
    let data = &pdf[i..end];
    if !flate {
        return (data.len() <= MAX_XML && is_cdxml(data)).then(|| String::from_utf8_lossy(data).into_owned());
    }
    use miniz_oxide::inflate::{decompress_to_vec_zlib_with_limit as inflate, TINFLStatus};
    // Look at the start first, so a large stream that is not CDXML is never inflated in full.
    let head = match inflate(data, 1024) {
        Ok(v) => v,
        Err(e) if e.status == TINFLStatus::HasMoreOutput => e.output,
        Err(_) => return None,
    };
    if !is_cdxml(&head) {
        return None;
    }
    inflate(data, MAX_XML).ok().map(|v| String::from_utf8_lossy(&v).into_owned())
}

fn find(h: &[u8], n: &[u8]) -> Option<usize> {
    h.windows(n.len()).position(|w| w == n)
}

fn rfind(h: &[u8], n: &[u8]) -> Option<usize> {
    h.windows(n.len()).rposition(|w| w == n)
}

fn is_white(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\r' | b'\n' | b'\x0c' | 0)
}

fn is_delim(c: u8) -> bool {
    matches!(c, b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%')
}

// Past whitespace and comments.
fn skip_white(b: &[u8], mut i: usize) -> usize {
    while let Some(&c) = b.get(i) {
        if is_white(c) {
            i += 1;
        } else if c == b'%' {
            while b.get(i).is_some_and(|&c| c != b'\n' && c != b'\r') {
                i += 1;
            }
        } else {
            break;
        }
    }
    i
}

// The regular characters from `i` up to whitespace or a delimiter.
fn word(b: &[u8], i: usize) -> &[u8] {
    let Some(rest) = b.get(i..) else { return &[] };
    &rest[..rest.iter().position(|&c| is_white(c) || is_delim(c)).unwrap_or(rest.len())]
}

fn int(b: &[u8], i: usize) -> Option<(u64, usize)> {
    let w = word(b, i);
    if w.is_empty() || w.len() > 12 || !w.iter().all(u8::is_ascii_digit) {
        return None;
    }
    Some((std::str::from_utf8(w).ok()?.parse().ok()?, i + w.len()))
}

// "N G R" at `i`: the object number, and where the reference ends.
fn reference(b: &[u8], i: usize) -> Option<(u32, usize)> {
    let (n, j) = int(b, skip_white(b, i))?;
    let (_, j) = int(b, skip_white(b, j))?;
    let j = skip_white(b, j);
    if word(b, j) != b"R" {
        return None;
    }
    Some((u32::try_from(n).ok()?, j + 1))
}

// Whether the "obj" at `i` follows "N G " (and so is not "endobj").
fn obj_header(b: &[u8], i: usize) -> bool {
    let back_white = |mut j: usize| {
        while j > 0 && is_white(b[j - 1]) {
            j -= 1;
        }
        j
    };
    let back_digits = |mut j: usize| {
        while j > 0 && b[j - 1].is_ascii_digit() {
            j -= 1;
        }
        j
    };
    let after = b.get(i + 3).is_none_or(|&c| is_white(c) || is_delim(c));
    let j = back_white(i);
    let k = back_digits(j);
    let l = back_white(k);
    let m = back_digits(l);
    after && j < i && k < j && l < k && m < l && (m == 0 || is_white(b[m - 1]))
}

// Where object `n`'s body starts (after "n g obj"). The last definition wins, as in an updated PDF.
fn object(b: &[u8], n: u32) -> Option<usize> {
    let pat = format!("{n} ");
    let pat = pat.as_bytes();
    let mut found = None;
    let mut from = 0;
    while let Some(k) = b.get(from..).and_then(|rest| find(rest, pat)) {
        let i = from + k;
        from = i + 1;
        if i == 0 || !is_white(b[i - 1]) {
            continue;
        }
        if let Some((_, j)) = int(b, skip_white(b, i + pat.len())) {
            let j = skip_white(b, j);
            if word(b, j) == b"obj" {
                found = Some(j + 3);
            }
        }
    }
    found
}

enum Val<'a> {
    Name(&'a [u8]),
    Int(u64),
    Ref(u32),
    Array(&'a [u8]),
    Other,
}

fn get<'a, 'b>(d: &'b [(&'a [u8], Val<'a>)], key: &[u8]) -> Option<&'b Val<'a>> {
    d.iter().find(|(k, _)| *k == key).map(|(_, v)| v)
}

// The names in an array's bytes, such as [/FlateDecode].
fn names(a: &[u8]) -> Vec<&[u8]> {
    a.split(|&c| c == b'/').skip(1).map(|s| word(s, 0)).collect()
}

// The dictionary starting at `at` (after whitespace): its entries, and where it ends.
fn dict(b: &[u8], at: usize) -> Option<(Vec<(&[u8], Val<'_>)>, usize)> {
    let b = &b[..b.len().min(at.saturating_add(16 << 10))];
    let mut i = skip_white(b, at);
    if b.get(i..i + 2)? != b"<<" {
        return None;
    }
    i += 2;
    let mut out = Vec::new();
    loop {
        i = skip_white(b, i);
        if b.get(i..i + 2)? == b">>" {
            return Some((out, i + 2));
        }
        if *b.get(i)? != b'/' {
            return None;
        }
        let key = word(b, i + 1);
        i = skip_white(b, i + 1 + key.len());
        let (val, next) = value(b, i)?;
        out.push((key, val));
        i = next;
    }
}

fn value(b: &[u8], i: usize) -> Option<(Val<'_>, usize)> {
    match *b.get(i)? {
        b'/' => {
            let w = word(b, i + 1);
            Some((Val::Name(w), i + 1 + w.len()))
        }
        b'<' if b.get(i + 1) == Some(&b'<') => Some((Val::Other, skip_nested(b, i)?)),
        b'[' => {
            let end = skip_nested(b, i)?;
            Some((Val::Array(&b[i + 1..end - 1]), end))
        }
        b'<' => Some((Val::Other, i + 1 + b.get(i + 1..)?.iter().position(|&c| c == b'>')? + 1)),
        b'(' => Some((Val::Other, skip_string(b, i)?)),
        c if c.is_ascii_digit() => {
            if let Some((r, end)) = reference(b, i) {
                return Some((Val::Ref(r), end));
            }
            if let Some((n, end)) = int(b, i) {
                return Some((Val::Int(n), end));
            }
            Some((Val::Other, i + word(b, i).len().max(1)))
        }
        _ => {
            let w = word(b, i);
            (!w.is_empty()).then_some((Val::Other, i + w.len()))
        }
    }
}

// Past a literal string "( ... )", with nested brackets and backslash escapes.
fn skip_string(b: &[u8], mut i: usize) -> Option<usize> {
    let mut depth = 0usize;
    loop {
        match *b.get(i)? {
            b'\\' => i += 1,
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
        i += 1;
    }
}

// Past a nested dictionary "<< ... >>" or array "[ ... ]", counting depth instead of recursing.
fn skip_nested(b: &[u8], mut i: usize) -> Option<usize> {
    let mut depth = 0usize;
    loop {
        match *b.get(i)? {
            b'(' => i = skip_string(b, i)?,
            b'<' if b.get(i + 1) == Some(&b'<') => {
                depth += 1;
                i += 2;
            }
            b'<' => i += 1 + b.get(i + 1..)?.iter().position(|&c| c == b'>')? + 1,
            b'>' if b.get(i + 1) == Some(&b'>') => {
                depth = depth.checked_sub(1)?;
                i += 2;
                if depth == 0 {
                    return Some(i);
                }
            }
            b'[' => {
                depth += 1;
                i += 1;
            }
            b']' => {
                depth = depth.checked_sub(1)?;
                i += 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            b'%' => i = skip_white(b, i),
            _ => i += 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="86pt" height="65pt" viewBox="0 0 86 65">
        <path d="M10 10L60 10L60 12L10 12Z" fill="rgb(15, 23, 42)"/>
        <text x="20" y="40" font-family="Arial, Helvetica, sans-serif" font-size="10" fill="rgb(15, 23, 42)">NH<tspan baseline-shift="sub" font-size="7.5">2</tspan></text>
    </svg>"##;
    const CDXML: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" ?>\n<!DOCTYPE CDXML SYSTEM \"http://www.cambridgesoft.com/xml/cdxml.dtd\" >\n<CDXML><page><fragment/></page></CDXML>\n";

    #[test]
    fn pdf_carries_cdxml_and_reads_back() {
        let pdf = make_pdf(SVG, Some(CDXML)).unwrap();
        assert_eq!(cdxml_from_pdf(&pdf).as_deref(), Some(CDXML));
        // The catalog points at the stream, which is marked as ChemDraw marks its own.
        let text = String::from_utf8_lossy(&pdf);
        assert!(text.contains("/Type /Catalog"));
        assert!(find(&pdf, b"/Metadata").is_some() && find(&pdf, b"/Subtype /XML").is_some() && find(&pdf, b"/Vendor /org.octet").is_some());
        // True size: 86 x 65 pt.
        assert!(text.contains("/MediaBox [0 0 86 65]"), "{}", &text[..text.len().min(2000)]);
        // Labels are text in an embedded font subset.
        assert!(text.contains("/FontFile2"));
        #[cfg(target_os = "macos")]
        assert!(text.contains("+ArialMT"));
    }

    // PackBits read back (TIFF 6.0, section 9).
    fn unpack(data: &[u8]) -> Vec<u8> {
        let (mut out, mut i) = (Vec::new(), 0);
        while i < data.len() {
            let n = data[i] as i8;
            i += 1;
            if n >= 0 {
                out.extend_from_slice(&data[i..i + n as usize + 1]);
                i += n as usize + 1;
            } else if n != -128 {
                out.extend(std::iter::repeat_n(data[i], (1 - n as i32) as usize));
                i += 1;
            }
        }
        out
    }

    #[test]
    fn packbits_reads_back() {
        for line in [vec![], vec![7], vec![1, 2, 3], vec![9; 300], [vec![1, 2], vec![5; 3], vec![6, 7, 8], vec![0; 129]].concat(), (0..=255).collect::<Vec<u8>>()] {
            let mut out = Vec::new();
            packbits(&line, &mut out);
            assert_eq!(unpack(&out), line);
        }
    }

    #[test]
    fn rasters_have_their_size_and_resolution() {
        // 86 x 65 pt at 144 dpi: 172 x 130 px.
        let jpeg = make_jpeg(SVG, 144.0).unwrap();
        let img = image::load_from_memory(&jpeg).unwrap();
        assert_eq!((img.width(), img.height()), (172, 130));
        let gif = make_gif(SVG, 144.0, true).unwrap();
        assert!(gif.starts_with(b"GIF89a"));
        assert_eq!(image::load_from_memory(&gif).unwrap().width(), 172);
        let tiff = make_tiff(SVG, 144.0, true).unwrap();
        assert!(tiff.starts_with(b"II*\0"));
        let u16_at = |at: usize| u16::from_le_bytes([tiff[at], tiff[at + 1]]);
        let u32_at = |at: usize| u32::from_le_bytes(tiff[at..at + 4].try_into().unwrap());
        let dir = u32_at(4) as usize;
        let tags: Vec<(u16, u32, u32)> = (0..u16_at(dir) as usize).map(|k| (u16_at(dir + 2 + 12 * k), u32_at(dir + 6 + 12 * k), u32_at(dir + 10 + 12 * k))).collect();
        let tag = |t: u16| tags.iter().find(|x| x.0 == t).copied().unwrap();
        assert_eq!((tag(256).2, tag(257).2), (172, 130));
        assert_eq!(tag(259).2, 32773);
        assert_eq!(tag(338).2, 2);
        let res = tag(282).2 as usize;
        assert_eq!((u32_at(res), u32_at(res + 4)), (14400, 100));
        // The strips unpack to 4 bytes a pixel, 16 rows each but the last.
        let (offs, lens) = (tag(273), tag(279));
        let strips = offs.1 as usize;
        let mut pixels = 0;
        for k in 0..strips {
            let (o, n) = (u32_at(offs.2 as usize + 4 * k) as usize, u32_at(lens.2 as usize + 4 * k) as usize);
            pixels += unpack(&tiff[o..o + n]).len() / 4;
        }
        assert_eq!(pixels, 172 * 130);
        if let Ok(dir) = std::env::var("OCTET_EXPORT_SAMPLES") {
            for (name, bytes) in [("sample.tif", tiff), ("sample.jpg", jpeg), ("sample.gif", gif), ("sample-white.tif", make_tiff(SVG, 144.0, false).unwrap()),
                ("sample.eps", crate::vector::make_eps(&tree(SVG).unwrap())), ("sample.emf", crate::vector::make_emf(&tree(SVG).unwrap(), None))] {
                std::fs::write(std::path::Path::new(&dir).join(name), bytes).unwrap();
            }
        }
    }

    #[test]
    fn cdxml_takes_chemdraws_header() {
        let doc = cdxml_document("\u{feff}<?xml version=\"1.0\"?>\n<!DOCTYPE CDXML SYSTEM \"x\">\n<CDXML/>");
        assert!(doc.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\" ?>\n<!DOCTYPE CDXML SYSTEM \"http://www.cambridgesoft.com/xml/cdxml.dtd\" >\n<CDXML/>"));
        assert_eq!(cdxml_document("<CDXML/>"), doc);
    }

    #[test]
    fn pdf_without_cdxml_has_no_metadata() {
        let pdf = make_pdf(SVG, None).unwrap();
        assert!(find(&pdf, b"/Metadata").is_none());
        assert_eq!(cdxml_from_pdf(&pdf), None);
    }

    #[test]
    fn finds_marked_stream_without_catalog_entry() {
        // An uncompressed metadata stream that the catalog does not name, with an indirect /Length.
        let body = CDXML.as_bytes();
        let pdf = [
            b"%PDF-1.3\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".as_slice(),
            b"5 0 obj\n<< /Length 6 0 R /Type /Metadata /Subtype /XML >>\nstream\n",
            body,
            format!("\nendstream\nendobj\n6 0 obj\n{}\nendobj\ntrailer\n<< /Size 7 /Root 1 0 R >>\n%%EOF\n", body.len()).as_bytes(),
        ].concat();
        assert_eq!(cdxml_from_pdf(&pdf).as_deref(), Some(CDXML));
    }

    #[test]
    fn hostile_pdfs_are_refused_quietly() {
        for pdf in [
            b"%PDF-1.4\n1 0 obj << /Metadata 1 0 R".as_slice(),
            b"%PDF trailer << /Root 99999999999 0 R >>",
            b"%PDF\n1 0 obj << /Type /Metadata /Subtype /XML /Length 999999 /Filter /FlateDecode >>\nstream\nxx",
            b"%PDF\n1 0 obj << /A << /B << [ ( ) ) ] >> >> /Type",
            b"%PDF\n1 0 obj << /Type /Metadata /Subtype /XML /Filter [/LZWDecode] /Length 3 >>\nstream\nabc\nendstream",
            b"%PDF octet-cdxml-base64:!!!",
        ] {
            assert_eq!(cdxml_from_pdf(pdf), None);
        }
        // A zip bomb that starts like CDXML stops at the 32 MB cap.
        let mut big = CDXML.as_bytes().to_vec();
        big.resize(40 << 20, b' ');
        let bomb = miniz_oxide::deflate::compress_to_vec_zlib(&big, 1);
        let pdf = [b"%PDF\n1 0 obj << /Type /Metadata /Subtype /XML /Filter /FlateDecode >>\nstream\n".as_slice(), &bomb, b"\nendstream"].concat();
        assert_eq!(cdxml_from_pdf(&pdf), None);
    }

    // REL-25: thousands of marked streams that never end, or whose /Length points at an object, then a long tail.
    // Each search used to read to the end of the file, so 10,000 of them took minutes on 63 MB.
    #[test]
    fn crafted_pdfs_end_quickly() {
        for length in ["", "/Length 7 0 R "] {
            let mut pdf = b"%PDF-1.4\n".to_vec();
            for k in 0..10_000 {
                pdf.extend_from_slice(format!("{} 0 obj << /Type /Metadata /Subtype /XML {length}>>\nstream\n", k + 1).as_bytes());
            }
            pdf.resize(8 << 20, b'x');
            let t = std::time::Instant::now();
            assert_eq!(cdxml_from_pdf(&pdf), None);
            assert!(t.elapsed() < std::time::Duration::from_secs(5), "{length:?} took {:?}", t.elapsed());
        }
        // A sound stream after many that are not CDXML is still found.
        let mut pdf = b"%PDF-1.4\n".to_vec();
        for k in 0..100 {
            pdf.extend_from_slice(format!("{} 0 obj << /Type /Metadata /Subtype /XML >>\nstream\n<x/>\nendstream\nendobj\n", k + 1).as_bytes());
        }
        pdf.extend_from_slice(format!("500 0 obj << /Type /Metadata /Subtype /XML /Length 999 >>\nstream\n{CDXML}\nendstream\nendobj\n").as_bytes());
        assert_eq!(cdxml_from_pdf(&pdf).as_deref(), Some(CDXML));
        // Every /Length points at the start of one long run of spaces, of a comment, or of letters, so each check for
        // "endstream" there used to read all of the run.
        for fill in [b" ".as_slice(), b"%x", b"x"] {
            let mut pdf = b"%PDF-1.4\n".to_vec();
            let mut starts = Vec::new();
            for k in 0..10_000 {
                pdf.extend_from_slice(format!("{} 0 obj << /Type /Metadata /Subtype /XML /Length 0000000000 >>\nstream\n", k + 1).as_bytes());
                starts.push(pdf.len());
            }
            let run = pdf.len();
            for &s in &starts {
                let at = s - "0000000000 >>\nstream\n".len();
                pdf[at..at + 10].copy_from_slice(format!("{:010}", run - s).as_bytes());
            }
            pdf.extend_from_slice(fill);
            pdf.resize(8 << 20, *fill.last().unwrap());
            let t = std::time::Instant::now();
            assert_eq!(cdxml_from_pdf(&pdf), None);
            assert!(t.elapsed() < std::time::Duration::from_secs(5), "{fill:?} took {:?}", t.elapsed());
        }
    }

    // A picture in the drawing (dropped, pasted, or from a ChemDraw file) shows in every export: the page writes it as
    // an <image> with its PNG, JPEG or GIF bytes in a data: address.
    #[test]
    fn pictures_show_in_exports() {
        use base64::Engine;
        let mut png_bytes = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut png_bytes, 8, 4);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            enc.write_header().unwrap().write_image_data(&[255, 0, 0, 255].repeat(32)).unwrap();
        }
        let red = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(8, 4, image::Rgb([255, 0, 0])));
        let made = |f: image::ImageFormat| {
            let mut out = std::io::Cursor::new(Vec::new());
            red.write_to(&mut out, f).unwrap();
            out.into_inner()
        };
        for (mime, bytes) in [("png", png_bytes), ("jpeg", made(image::ImageFormat::Jpeg)), ("gif", made(image::ImageFormat::Gif))] {
            let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
            let svg = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="40pt" height="20pt" viewBox="0 0 40 20"><image href="data:image/{mime};base64,{b64}" x="0" y="0" width="40" height="20" preserveAspectRatio="none"/></svg>"#);
            let (w, h, rgba) = pixels(&svg, 72.0, true).unwrap();
            let k = ((h / 2) * w + w / 2) as usize * 4;
            assert!(rgba[k] > 200 && rgba[k + 1] < 60 && rgba[k + 3] > 200, "{mime}: {:?}", &rgba[k..k + 4]);
            assert!(find(&make_pdf(&svg, None).unwrap(), b"/Subtype /Image").is_some(), "{mime}");
            // The clipboard's PNG on Windows and Linux, and EMF and EPS (vector.rs, which checks where they go).
            let mut reader = png::Decoder::new(std::io::Cursor::new(clipboard_pictures(&svg, false).unwrap().0)).read_info().unwrap();
            let mut buf = vec![0; reader.output_buffer_size()];
            let info = reader.next_frame(&mut buf).unwrap();
            let k = ((info.height / 2) * info.width + info.width / 2) as usize * 4;
            assert!(buf[k] > 200 && buf[k + 1] < 60 && buf[k + 3] > 200, "{mime}: {:?}", &buf[k..k + 4]);
            assert!(String::from_utf8(crate::vector::make_eps(&tree(&svg).unwrap())).unwrap().contains(" image\n"), "{mime}");
            assert!(find(&crate::vector::make_emf(&tree(&svg).unwrap(), None), &[81, 0, 0, 0]).is_some(), "{mime}");
        }
    }

    #[test]
    fn png_is_true_size_at_the_resolution() {
        let png = make_png(SVG, 300.0, true).unwrap();
        let header = png::Decoder::new(std::io::Cursor::new(png)).read_info().unwrap().info().clone();
        // 86 pt at 300 dpi.
        assert_eq!((header.width, header.height), (359, 271));
        assert_eq!(header.pixel_dims.unwrap().xppu, 11811);
    }

    // Sam's clipboard dumps (ChemDraw's copy, and it copied back out of Word). Set OCTET_CLIPBOARD_DUMPS to the
    // folder holding chemdraw-copy/ and word-copy/ to run this; the files stay out of the repository.
    #[test]
    fn reads_chemdraws_pdf() {
        let Ok(dir) = std::env::var("OCTET_CLIPBOARD_DUMPS") else { return };
        for sub in ["chemdraw-copy", "word-copy"] {
            let pdf = std::fs::read(format!("{dir}/{sub}/0_com.adobe.pdf")).unwrap();
            let x = cdxml_from_pdf(&pdf).unwrap();
            assert!(x.contains("<CDXML"), "{sub}");
        }
        let old = std::fs::read(format!("{dir}/word-octet-pdf/0_com.adobe.pdf")).unwrap();
        assert!(cdxml_from_pdf(&old).unwrap().contains("<CDXML"));
    }
}
