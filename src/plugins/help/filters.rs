use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub fn get_text(locale: &str) -> String {
    crate::i18n::get_text(locale, "help.filters_text")
}

pub fn get_keyboard(locale: &str) -> InlineKeyboardMarkup {
    let back_text = crate::i18n::get_text(locale, "help.back");
    let back_text = if back_text == "help.back" { "🔙 Back".to_string() } else { format!("🔙 {}", back_text) };

    InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback(back_text, "help:back")],
    ])
}
