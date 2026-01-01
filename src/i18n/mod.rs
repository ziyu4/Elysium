//! Internationalization (i18n) module.
//!
//! Handles loading translations and resolving locale based on context.

use std::collections::HashMap;
use std::sync::OnceLock;
use serde_json::Value;

/// Global translation store: LangCode -> Key -> Text
static TRANSLATIONS: OnceLock<HashMap<String, Value>> = OnceLock::new();

/// Initialize translations (load from JSON files).
/// In a real app, this might stick to `lazy_static` or load at startup.
/// For simplicity, we hardcode/embed the strings or load them here.
/// To follow the user's request, we'll use `include_str!` for simplicity and performance (no file I/O at runtime).
pub fn init() {
    let mut map = HashMap::new();
    
    // Load English
    let en_json = include_str!("en.json");
    if let Ok(val) = serde_json::from_str(en_json) {
        map.insert("en".to_string(), val);
    }
    
    // Load Indonesian
    let id_json = include_str!("id.json");
    if let Ok(val) = serde_json::from_str(id_json) {
        map.insert("id".to_string(), val);
    }

    let _ = TRANSLATIONS.set(map);
}

/// Get text for a key in a specific language.
/// Supports nested keys via dot notation, e.g., "help.welcome".
pub fn get_text(lang: &str, key: &str) -> String {
    let store = TRANSLATIONS.get();
    if store.is_none() {
        return key.to_string(); // Fallback if not init
    }
    let store = store.unwrap();

    // Try requested language
    if let Some(val) = store.get(lang) {
        if let Some(text) = resolve_key(val, key) {
            return text;
        }
    }

    // Fallback to "en"
    if lang != "en" {
        if let Some(val) = store.get("en") {
            if let Some(text) = resolve_key(val, key) {
                return text;
            }
        }
    }

    // Key not found
    key.to_string()
}

fn resolve_key(val: &Value, key: &str) -> Option<String> {
    let mut current = val;
    for part in key.split('.') {
        match current.get(part) {
            Some(v) => current = v,
            None => return None,
        }
    }
    current.as_str().map(|s| s.to_string())
}

/// Resolve effective locale.
/// Priority: Group Config -> User Config -> Default (id as per user request? or en?)
/// User said: "integrasikan bahasa inggris sebagai bahasa utama" (Integrate English as main language).
/// User also said: "Indonesian default for them?" implied by "bahasa utama" usually means "primary/default"?
/// But user text: "bahasa inggris sebagai bahasa utama" -> English is main.
/// Let's default to "en" but support "id".
pub fn resolve_locale(
    group_lang: Option<&str>,
    user_lang: Option<&str>,
) -> String {
    if let Some(l) = group_lang {
        return l.to_string();
    }
    if let Some(l) = user_lang {
        return l.to_string();
    }
    "en".to_string()
}
