use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub fn get_text() -> String {
    "<b>👋 Bantuan: Goodbye</b>\n\n\
    Fitur goodbye mengirim pesan otomatis saat member keluar dari grup.\n\n\
    <b>Perintah:</b>\n\
    • <code>/bye</code> - Lihat status & preview\n\
    • <code>/bye on/off</code> - Aktifkan/nonaktifkan\n\
    • <code>/setbye</code> - Atur pesan (reply ke pesan)\n\
    • <code>/setbyebuttons</code> - Atur tombol\n\
    • <code>/resetbye</code> - Reset ke default\n\n\
    <b>Format Tombol:</b>\n\
    Sama dengan welcome. Gunakan <code>{button:Teks|URL}</code>\n\n\
    <b>Placeholder:</b>\n\
    • <code>{first}</code>, <code>{last}</code>, <code>{fullname}</code>\n\
    • <code>{mention}</code>, <code>{id}</code>\n\
    • <code>{group}</code>, <code>{count}</code>"
    .to_string()
}

pub fn get_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback("🔙 Kembali", "help:back")],
    ])
}
