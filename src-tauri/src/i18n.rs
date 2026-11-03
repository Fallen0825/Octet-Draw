// Korean menu labels for the unofficial Korean build. The table is src/i18n/ko.json: keys are the English labels
// (without a trailing …), values the Korean. A menu item shows "한글 (English)", with the … after it; a value that
// starts with ! is shown as it is; menu bar titles ("&File") take their value as it is ("파일(&F)"). An English label
// with no entry stays English.
use std::{collections::HashMap, sync::OnceLock};

const KO: &str = include_str!("../../src/i18n/ko.json");

struct Table {
    menubar: HashMap<String, String>,
    menu: HashMap<String, String>,
}

fn table() -> &'static Table {
    static T: OnceLock<Table> = OnceLock::new();
    T.get_or_init(|| {
        let v: serde_json::Value = serde_json::from_str(KO).unwrap_or_default();
        let part = |name: &str| -> HashMap<String, String> {
            v.get(name)
                .and_then(|m| m.as_object())
                .map(|m| m.iter().filter_map(|(k, s)| s.as_str().map(|s| (k.clone(), s.to_string()))).collect())
                .unwrap_or_default()
        };
        Table { menubar: part("menubar"), menu: part("menu") }
    })
}

pub fn tr(en: &str) -> String {
    let t = table();
    if let Some(ko) = t.menubar.get(en) {
        return ko.clone();
    }
    let (base, ellipsis) = match en.strip_suffix('…') {
        Some(b) => (b, "…"),
        None => (en, ""),
    };
    match t.menu.get(base) {
        Some(ko) if ko.starts_with('!') => format!("{}{ellipsis}", &ko[1..]),
        Some(ko) if ko != base => format!("{ko} ({base}){ellipsis}"),
        _ => en.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::tr;

    #[test]
    fn korean_labels() {
        assert_eq!(tr("&File"), "파일(&F)");
        assert_eq!(tr("Save As…"), "다른 이름으로 저장 (Save As)…");
        assert_eq!(tr("Undo"), "실행 취소 (Undo)");
        assert_eq!(tr("CDXML Text"), "CDXML 텍스트");
        assert_eq!(tr("SMILES"), "SMILES");
        assert_eq!(tr("My own colour"), "My own colour");
    }
}
