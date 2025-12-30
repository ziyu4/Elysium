use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub fn get_text() -> &'static str {
    "<b>📚 Menu Bantuan Elysium</b>\n\n\
    Silakan pilih kategori bantuan di bawah ini untuk melihat daftar perintah dan cara penggunaannya."
}

pub fn get_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::callback("📝 Notes", "help:notes"),
            InlineKeyboardButton::callback("⚙️ Filters", "help:filters"),
        ],
        vec![
            InlineKeyboardButton::callback("👋 Welcome", "help:welcome"),
            InlineKeyboardButton::callback("👋 Goodbye", "help:bye"),
        ],
        vec![
            InlineKeyboardButton::callback("🛡️ Admin", "help:admin"),
            InlineKeyboardButton::callback("⚠️ Warns", "help:warns"),
        ],
        vec![
            InlineKeyboardButton::callback("🌊 Antiflood", "help:antiflood"),
            InlineKeyboardButton::callback("✅ Approval", "help:approval"),
        ],
        vec![
            InlineKeyboardButton::callback("💤 AFK", "help:afk"),
            InlineKeyboardButton::callback("🗑️ Purge", "help:purge"),
        ],
        vec![
            InlineKeyboardButton::callback("📜 Rules", "help:rules"),
        ],
    ])
}
