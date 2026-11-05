//! `dune config get|set`: the look settings of `dune.toml` read and written from outside
//! (dune-shell's settings window). `set` edits the file with toml_edit, so its comments
//! and order stay, then asks dune to apply it at once (`Request::ReloadConfig`); saving the
//! file would apply it too, half a second later (`crate::reload`).

use std::time::Duration;

use dune_common::protocol::{Request, ask};
use toml_edit::{DocumentMut, Item, value};

#[derive(Clone, Copy)]
enum Kind {
    /// `#RRGGBB` or `#RRGGBBAA`; `none` too when set.
    Color { none: bool },
    /// Whole numbers in this range.
    Number(i64, i64),
    /// Decimals in this range.
    Decimal(f64, f64),
    Bool,
}

/// The settings `get` and `set` know (`table.key` for those in a table), with what they
/// take and their default.
const LOOK: [(&str, Kind, &str); 18] = [
    ("border_color", Kind::Color { none: true }, "#ffffff"),
    ("focus_color", Kind::Color { none: true }, "#5c9ef2"),
    ("background_color", Kind::Color { none: false }, "#14141a"),
    ("border_width", Kind::Number(0, 16), "1"),
    ("gap", Kind::Number(0, 64), "6"),
    ("corner_radius", Kind::Number(0, 64), "0"),
    ("fill_on_close", Kind::Bool, "false"),
    ("tabs.height", Kind::Number(16, 64), "24"),
    ("tabs.font_size", Kind::Number(8, 32), "14"),
    ("tabs.background", Kind::Color { none: false }, "#1e2029"),
    ("tabs.active", Kind::Color { none: false }, "#4c566a"),
    ("tabs.text", Kind::Color { none: false }, "#d8dce6"),
    ("tabs.inactive_text", Kind::Color { none: false }, "#9096a4"),
    ("animations.enabled", Kind::Bool, "true"),
    ("animations.resize_ms", Kind::Number(0, 1000), "150"),
    ("animations.slide_ms", Kind::Number(0, 1000), "280"),
    ("animations.close_ms", Kind::Number(0, 1000), "150"),
    ("animations.close_scale", Kind::Decimal(0.5, 1.0), "0.85"),
];

pub fn config_command(args: &[String]) -> Result<(), String> {
    let path = crate::config::path().ok_or("no config folder ($XDG_CONFIG_HOME or $HOME)")?;
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    match args {
        [get] if get == "get" => {
            println!("{}", look(&text)?);
            Ok(())
        }
        [set, key, val] if set == "set" => {
            let new = set_in(&text, key, val)?;
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            }
            // Whole or not at all: dune never reads half a file.
            let tmp = path.with_extension("toml.tmp");
            std::fs::write(&tmp, new).and_then(|_| std::fs::rename(&tmp, &path)).map_err(|e| format!("{}: {e}", path.display()))?;
            // Not running (or another dune): the file is written, it applies when dune reads it.
            let _ = ask(&Request::ReloadConfig, Duration::from_secs(1));
            Ok(())
        }
        _ => Err("usage: dune config get | dune config set KEY VALUE".into()),
    }
}

/// The look settings in `text` as a JSON object by their `LOOK` names, each a string
/// (colors), a number or a boolean, at its default when unset or not one dune takes.
fn look(text: &str) -> Result<String, String> {
    let doc: DocumentMut = text.parse().map_err(|e| format!("dune.toml is not TOML: {e}"))?;
    let mut out = serde_json::Map::new();
    for (key, kind, default) in LOOK {
        let set = find(&doc, key).map(|v| match v {
            toml_edit::Value::String(s) => s.value().clone(),
            toml_edit::Value::Integer(n) => n.value().to_string(),
            toml_edit::Value::Float(f) => f.value().to_string(),
            toml_edit::Value::Boolean(b) => b.value().to_string(),
            _ => String::new(),
        });
        let text = set.and_then(|v| checked(kind, &v).ok()).unwrap_or_else(|| default.to_string());
        out.insert(key.to_string(), json(kind, &text));
    }
    Ok(serde_json::Value::Object(out).to_string())
}

/// The value at `key` (`table.key` inside a table), if it is set.
fn find<'a>(doc: &'a DocumentMut, key: &str) -> Option<&'a toml_edit::Value> {
    match key.split_once('.') {
        Some((table, leaf)) => doc.get(table)?.as_table_like()?.get(leaf)?.as_value(),
        None => doc.get(key)?.as_value(),
    }
}

/// `text`, checked as `kind`, the way JSON and TOML hold it.
fn json(kind: Kind, text: &str) -> serde_json::Value {
    match kind {
        Kind::Color { .. } => serde_json::Value::String(text.into()),
        Kind::Number(..) => serde_json::Value::from(text.parse::<i64>().unwrap_or_default()),
        Kind::Decimal(..) => serde_json::Value::from(text.parse::<f64>().unwrap_or_default()),
        Kind::Bool => serde_json::Value::Bool(text == "true"),
    }
}

/// `value` as `kind` writes it (`#rrggbb`, `none`, a number), or why dune would not take it.
fn checked(kind: Kind, value: &str) -> Result<String, String> {
    let value = value.trim();
    match kind {
        Kind::Color { none } => {
            if none && value.eq_ignore_ascii_case("none") {
                return Ok("none".into());
            }
            let hex = value.trim_start_matches('#');
            if matches!(hex.len(), 6 | 8) && hex.chars().all(|c| c.is_ascii_hexdigit()) {
                Ok(format!("#{}", hex.to_ascii_lowercase()))
            } else {
                Err(format!("`{value}` is not #RRGGBB or #RRGGBBAA{}", if none { " or none" } else { "" }))
            }
        }
        Kind::Number(min, max) => match value.parse::<i64>() {
            Ok(n) if (min..=max).contains(&n) => Ok(n.to_string()),
            _ => Err(format!("`{value}` is not a number from {min} to {max}")),
        },
        Kind::Decimal(min, max) => match value.parse::<f64>() {
            // Two decimals are all anyone sees.
            Ok(n) if (min..=max).contains(&n) => Ok(((n * 100.0).round() / 100.0).to_string()),
            _ => Err(format!("`{value}` is not a number from {min} to {max}")),
        },
        Kind::Bool => match value {
            "true" | "false" => Ok(value.into()),
            _ => Err(format!("`{value}` is not true or false")),
        },
    }
}

/// `text` with `key` set to `val`: in its place with its comment when it was there, else
/// with the other settings of its table (the table added at the end when there is none).
fn set_in(text: &str, key: &str, val: &str) -> Result<String, String> {
    let Some((_, kind, _)) = LOOK.iter().find(|(k, ..)| *k == key) else {
        let keys: Vec<_> = LOOK.iter().map(|(k, ..)| *k).collect();
        return Err(format!("`{key}` is not one of {}", keys.join(", ")));
    };
    let checked = checked(*kind, val)?;
    let mut doc: DocumentMut = text.parse().map_err(|e| format!("dune.toml is not TOML, fix it first: {e}"))?;
    let new = match kind {
        Kind::Color { .. } => toml_edit::Value::from(checked),
        Kind::Number(..) => toml_edit::Value::from(checked.parse::<i64>().unwrap_or_default()),
        Kind::Decimal(..) => toml_edit::Value::from(checked.parse::<f64>().unwrap_or_default()),
        Kind::Bool => toml_edit::Value::from(checked == "true"),
    };
    let (table, leaf) = match key.split_once('.') {
        Some((table, leaf)) => {
            if doc.get(table).is_none() {
                doc[table] = toml_edit::table();
            }
            let table = doc[table].as_table_like_mut().ok_or(format!("`{table}` in dune.toml is not a table"))?;
            (table, leaf)
        }
        None => (doc.as_table_mut() as &mut dyn toml_edit::TableLike, key),
    };
    match table.get_mut(leaf).and_then(Item::as_value_mut) {
        Some(old) => {
            let decor = old.decor().clone();
            let mut new = new;
            *new.decor_mut() = decor;
            *old = new;
        }
        None => {
            table.insert(leaf, value(new));
        }
    }
    Ok(doc.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = "# my dune\nfont = \"x\"\ngap = 6   # roomy\n\n[keyboard]\nlayout = \"us\"\n";

    #[test]
    fn set_keeps_the_comments() {
        let new = set_in(FILE, "gap", "12").unwrap();
        assert_eq!(new, "# my dune\nfont = \"x\"\ngap = 12   # roomy\n\n[keyboard]\nlayout = \"us\"\n");
        let new = set_in(FILE, "focus_color", "FF0000").unwrap();
        assert!(new.starts_with("# my dune\nfont = \"x\"\ngap = 6   # roomy\nfocus_color = \"#ff0000\"\n"), "{new}");
        // What dune itself reads from it.
        let config = crate::config::from_text(&new);
        assert_eq!(config.focus_color, Some([1.0, 0.0, 0.0, 1.0]));
    }

    #[test]
    fn set_refuses_what_dune_would_not_take() {
        assert!(set_in(FILE, "gap", "65").is_err());
        assert!(set_in(FILE, "gap", "wide").is_err());
        assert!(set_in(FILE, "background_color", "none").is_err());
        assert!(set_in(FILE, "terminal", "foot").is_err());
        assert!(set_in("gap = [", "gap", "1").is_err());
        assert_eq!(set_in(FILE, "border_color", "None").unwrap().matches("border_color = \"none\"").count(), 1);
    }

    #[test]
    fn set_goes_into_tables() {
        let new = set_in(FILE, "tabs.height", "30").unwrap();
        assert!(new.ends_with("[keyboard]\nlayout = \"us\"\n\n[tabs]\nheight = 30\n"), "{new}");
        let new = set_in(&new, "tabs.height", "32").unwrap();
        assert_eq!(new.matches("height = 32").count(), 1);
        let new = set_in(&new, "animations.close_scale", "0.7").unwrap();
        let new = set_in(&new, "animations.enabled", "false").unwrap();
        let config = crate::config::from_text(&new);
        assert_eq!(config.tab_style.height, 32);
        assert_eq!((config.animations.close_scale, config.animations.resize), (0.7, std::time::Duration::ZERO));
        assert!(set_in(FILE, "animations.enabled", "yes").is_err());
        assert!(set_in(FILE, "animations.close_scale", "0.2").is_err());
        let json: serde_json::Value = serde_json::from_str(&look(&new).unwrap()).unwrap();
        assert_eq!((&json["tabs.height"], &json["animations.enabled"], &json["animations.close_scale"]), (&32.into(), &false.into(), &0.7.into()));
    }

    #[test]
    fn get_fills_in_the_defaults() {
        let json: serde_json::Value = serde_json::from_str(&look("gap = 12  # wide\nborder_color = \"none\"\nborder_width = 99\n").unwrap()).unwrap();
        assert_eq!(json["gap"], 12);
        assert_eq!(json["border_color"], "none");
        assert_eq!(json["border_width"], 1);
        assert_eq!(json["focus_color"], "#5c9ef2");
        assert_eq!(json["corner_radius"], 0);
    }
}
