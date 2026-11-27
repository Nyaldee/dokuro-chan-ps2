//! Les réglages du portage (le jeu d'origine n'enregistre rien du
//! mini-jeu) : le mode d'affichage, et dans « assets » le chemin de l'ISO
//! donné par le joueur et la marque de celle déjà vérifiée. En JSON dans
//! Documents\My Games\Game ni Natta yo! Dokuro-chan Kenkou Shindan
//! Daisakusen (~/.local/share sous Linux, « saves » à côté de l'exécutable
//! en version portable).
//!
//! ```json
//! {
//!   "displaysetting": "window",
//!   "assets": { "checked": "1487536128:1131573600", "path": "" }
//! }
//! ```

use std::collections::BTreeMap;
use std::path::PathBuf;

const FOLDER: &str = "Game ni Natta yo! Dokuro-chan Kenkou Shindan Daisakusen";
const FILE: &str = "save.json";

const ASSETS: &str = "assets";
/// L'ISO (fichier, ou dossier où la chercher) donnée par le joueur.
const PATH: &str = "path";
/// La marque (taille et date) de l'ISO déjà vérifiée.
const CHECKED: &str = "checked";

pub struct Save {
    path: Option<PathBuf>,
    values: BTreeMap<String, String>,
    assets: BTreeMap<String, String>,
}

impl Save {
    pub fn load() -> Save {
        let path = crate::platform::saves(FOLDER).map(|d| d.join(FILE));
        let text = path.as_ref().and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default();
        let (values, assets) = parse(&text);
        Save { path, values, assets }
    }

    /// Une sauvegarde lue mais jamais écrite (tests).
    pub fn memory() -> Save {
        let mut s = Save::load();
        s.path = None;
        s
    }

    /// Le fichier de la sauvegarde, pour le joueur.
    pub fn file(&self) -> String {
        self.path.as_ref().map_or_else(String::new, |p| p.display().to_string())
    }

    pub fn get(&self, key: &str) -> &str {
        self.values.get(key).map_or("", String::as_str)
    }

    pub fn set(&mut self, key: &str, value: &str) {
        if self.get(key) == value {
            return;
        }
        self.values.insert(key.to_string(), value.to_string());
        self.write();
    }

    /// Le chemin de l'ISO donné par le joueur, et la marque retenue.
    pub fn assets(&self) -> (&str, &str) {
        let get = |k| self.assets.get(k).map_or("", String::as_str);
        (get(PATH), get(CHECKED))
    }

    /// Retient la marque de l'ISO vérifiée ; « path » reste présent (vide)
    /// pour que le joueur le trouve.
    pub fn set_assets_checked(&mut self, stamp: &str) {
        if self.assets.contains_key(PATH) && self.assets().1 == stamp {
            return;
        }
        self.assets.entry(PATH.to_string()).or_default();
        self.assets.insert(CHECKED.to_string(), stamp.to_string());
        self.write();
    }

    fn write(&self) {
        let Some(path) = &self.path else { return };
        let pair = |indent: &str, (k, v): (&String, &String)| format!("{indent}{}: {}", quote(k), quote(v));
        let mut lines: Vec<String> = self.values.iter().map(|e| pair("  ", e)).collect();
        if !self.assets.is_empty() {
            let inner: Vec<String> = self.assets.iter().map(|e| pair("    ", e)).collect();
            lines.push(format!("  {}: {{\n{}\n  }}", quote(ASSETS), inner.join(",\n")));
        }
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(path, format!("{{\n{}\n}}\n", lines.join(",\n")));
    }
}

fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Un objet JSON de chaînes, et son objet « assets » de chaînes ; le reste
/// est ignoré (BOM toléré).
fn parse(text: &str) -> (BTreeMap<String, String>, BTreeMap<String, String>) {
    let (mut values, mut assets) = (BTreeMap::new(), BTreeMap::new());
    let mut chars = text.trim_start_matches('\u{feff}').chars().peekable();
    // La profondeur (1 : la racine), l'objet de la racine ouvert et la clé
    // qui attend sa valeur.
    let mut depth = 0;
    let mut group: Option<String> = None;
    let mut key: Option<String> = None;
    while let Some(c) = chars.next() {
        match c {
            '{' => {
                depth += 1;
                let k = key.take();
                if depth == 2 {
                    group = k;
                }
            }
            '}' => {
                depth -= 1;
                key = None;
                if depth <= 1 {
                    group = None;
                }
            }
            ',' => key = None,
            '"' => {
                let mut s = String::new();
                while let Some(c) = chars.next() {
                    match c {
                        '"' => break,
                        '\\' => match chars.next() {
                            Some('n') => s.push('\n'),
                            Some('u') => {
                                let hex: String = chars.by_ref().take(4).collect();
                                s.extend(u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32));
                            }
                            Some(c) => s.push(c),
                            None => {}
                        },
                        c => s.push(c),
                    }
                }
                match key.take() {
                    None => key = Some(s),
                    Some(k) if depth == 1 => {
                        values.insert(k, s);
                    }
                    Some(k) if depth == 2 && group.as_deref() == Some(ASSETS) => {
                        assets.insert(k, s);
                    }
                    Some(_) => {}
                }
            }
            _ => {}
        }
    }
    (values, assets)
}

#[cfg(test)]
mod tests {
    #[test]
    fn round_trip() {
        let (v, a) = super::parse("\u{feff}{\n  \"displaysetting\": \"borderless\",\n  \"n\": 3,\n  \"assets\": {\n    \"checked\": \"1:2\",\n    \"path\": \"D:\\\\iso\"\n  },\n  \"challenge\": \"cleared\"\n}\n");
        assert_eq!(v.get("displaysetting").map(String::as_str), Some("borderless"));
        assert_eq!(v.get("challenge").map(String::as_str), Some("cleared"));
        assert_eq!(v.len(), 2);
        assert_eq!(a.get("checked").map(String::as_str), Some("1:2"));
        assert_eq!(a.get("path").map(String::as_str), Some("D:\\iso"));
        assert_eq!(super::quote("a\"b"), "\"a\\\"b\"");
    }
}
