use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub fn get_text() -> String {
    "<b>📜 Bantuan: Rules</b>\n\n\
    Atur peraturan grup.\n\n\
    <b>Perintah:</b>\n\
    • <code>/rules</code> - Lihat peraturan grup\n\
    • <code>/setrules</code> - Atur peraturan (reply ke pesan)\n\
    • <code>/clearrules</code> - Hapus peraturan\n\
    • <code>/setrulesprivate on/off</code> - Kirim rules ke PM\n\n\
    <b>Format:</b>\n\
    Mendukung tombol dan placeholder seperti welcome.\n\
    Gunakan <code>{button:Teks|URL}</code> untuk tombol.\n\n\
    <b>Integrasi:</b>\n\
    Gunakan <code>{rules}</code> di welcome/notes untuk menyertakan rules."
    .to_string()
}

pub fn get_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback("🔙 Kembali", "help:back")],
    ])
}
