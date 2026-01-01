use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub fn get_text(locale: &str) -> String {
    let title = crate::i18n::get_text(locale, "help.title");
    let intro = crate::i18n::get_text(locale, "help.intro");
    format!("{}\n\n{}", title, intro)
}

pub fn get_keyboard(locale: &str) -> InlineKeyboardMarkup {
    // For now, hardcode button labels or use i18n if keys exist.
    // Ideally buttons should be translated too.
    // I'll stick to English for buttons unless requested, OR use keys if available.
    // "notes", "filters" etc are technically terms.
    // Let's use simple labels for now to match current behavior but prepared for i18n if I added keys.
    // User requested "casual" Indonesian. "Notes" -> "Catatan" (in json).
    
    let l = |key: &str, default: &str| -> String {
        let text = crate::i18n::get_text(locale, key);
        if text == key { default.to_string() } else { text }
    };

    InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::callback(l("help.notes", "📝 Notes"), "help:notes"),
            InlineKeyboardButton::callback(l("help.filters", "⚙️ Filters"), "help:filters"),
        ],
        vec![
            InlineKeyboardButton::callback(l("help.welcome", "👋 Welcome"), "help:welcome"),
            InlineKeyboardButton::callback(l("help.bye", "👋 Goodbye"), "help:bye"),
        ],
        vec![
            InlineKeyboardButton::callback(l("help.admin", "🛡️ Admin"), "help:admin"),
            InlineKeyboardButton::callback(l("help.warns", "⚠️ Warns"), "help:warns"),
        ],
        vec![
            InlineKeyboardButton::callback(l("help.antiflood", "🌊 Antiflood"), "help:antiflood"),
            InlineKeyboardButton::callback(l("help.approval", "✅ Approval"), "help:approval"),
        ],
        vec![
            InlineKeyboardButton::callback(l("help.afk", "💤 AFK"), "help:afk"),
            InlineKeyboardButton::callback(l("help.purge", "🗑️ Purge"), "help:purge"),
        ],
        vec![
            InlineKeyboardButton::callback(l("help.rules", "📜 Rules"), "help:rules"),
        ],
    ])
}
