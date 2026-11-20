//! Die Einstellungen der Mod, gespeichert in
//! `<Spielordner>/sunrise-mod-data/sunrise_tweaks/settings.txt` (eine Zeile `schlüssel=wert`).
//! Sie werden nur über das Klick-Menü geändert, nicht mehr auf der Config-Seite.

use std::collections::BTreeMap;

use mod_api::*;

const FILE: &str = "settings.txt";

#[derive(Default)]
pub struct Config {
    values: BTreeMap<String, String>,
    loaded: bool,
}

impl Config {
    /// Liest die Datei einmal ein (geht nur, solange eine Welt offen ist).
    pub fn ensure_loaded(&mut self, host: &dyn Host) {
        if self.loaded {
            return;
        }
        let Ok(path) = ConfigPath::new(FILE) else {
            return;
        };
        match host.data_files().read(&path) {
            Ok(bytes) => {
                self.values = parse(&String::from_utf8_lossy(&bytes));
                self.loaded = true;
            }
            Err(ConfigFileError::NotFound) => self.loaded = true,
            Err(_) => {}
        }
    }

    pub fn get_bool(&self, key: &str, default: bool) -> bool {
        match self.values.get(key).map(String::as_str) {
            Some("true") => true,
            Some("false") => false,
            _ => default,
        }
    }

    pub fn get_usize(&self, key: &str, default: usize) -> usize {
        self.values.get(key).and_then(|v| v.parse().ok()).unwrap_or(default)
    }

    pub fn get_f64(&self, key: &str, default: f64) -> f64 {
        self.values
            .get(key)
            .and_then(|v| v.parse::<f64>().ok())
            .filter(|v| v.is_finite())
            .unwrap_or(default)
    }

    /// Setzt einen Wert und speichert die Datei sofort.
    pub fn set(&mut self, host: &dyn Host, key: &str, value: String) {
        self.values.insert(key.to_owned(), value);
        if let Ok(path) = ConfigPath::new(FILE) {
            let _ = host.data_files().write(&path, render(&self.values).as_bytes());
        }
    }
}

fn parse(text: &str) -> BTreeMap<String, String> {
    text.lines()
        .filter_map(|line| line.split_once('='))
        .map(|(k, v)| (k.trim().to_owned(), v.trim().to_owned()))
        .filter(|(k, _)| !k.is_empty())
        .collect()
}

fn render(values: &BTreeMap<String, String>) -> String {
    values.iter().map(|(k, v)| format!("{k}={v}\n")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_survive_a_round_trip() {
        let mut values = BTreeMap::new();
        values.insert("autoclicker.cps".to_owned(), "12".to_owned());
        values.insert("autotool.enabled".to_owned(), "false".to_owned());
        assert_eq!(parse(&render(&values)), values);
    }

    #[test]
    fn missing_or_broken_values_fall_back_to_the_default() {
        let config = Config { values: parse("a=true\nb=xyz\nc=7\n"), loaded: true };
        assert!(config.get_bool("a", false));
        assert!(config.get_bool("b", true));
        assert_eq!(config.get_usize("c", 0), 7);
        assert_eq!(config.get_f64("missing", 10.0), 10.0);
    }
}
