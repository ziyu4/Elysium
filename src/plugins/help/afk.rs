use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub fn get_text() -> String {
    "<b>💤 Bantuan: AFK</b>\n\n\
    Fitur AFK (Away From Keyboard) memberi tahu user lain bahwa Anda sedang tidak aktif ketika mereka me-reply atau me-mention Anda.\n\n\
    <b>Perintah:</b>\n\
    • <code>/afk [alasan]</code> - Set status AFK\n\
    • <code>/brb [alasan]</code> - Alias untuk /afk\n\n\
    <b>Contoh:</b>\n\
    <code>/afk Sedang tidur</code>\n\
    <code>/brb Makan siang</code>\n\n\
    <b>Cara Kembali:</b>\n\
    Cukup kirim pesan apa saja di grup, status AFK akan otomatis hilang."
    .to_string()
}

pub fn get_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback("🔙 Kembali", "help:back")],
    ])
}
