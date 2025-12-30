use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub fn get_text() -> String {
    "<b>🌊 Bantuan: Antiflood</b>\n\n\
    Proteksi otomatis dari spam/flood pesan.\n\n\
    <b>Perintah:</b>\n\
    • <code>/antiflood</code> - Lihat status\n\
    • <code>/setflood [jumlah]</code> - Atur batas pesan\n\
    • <code>/setflood off</code> - Nonaktifkan\n\
    • <code>/setfloodpenalty [mode]</code> - Atur hukuman\n\n\
    <b>Mode Hukuman:</b>\n\
    • <code>warn</code> - Peringatan saja\n\
    • <code>mute</code> - Mute permanen\n\
    • <code>kick</code> - Kick dari grup\n\
    • <code>ban</code> - Ban permanen\n\
    • <code>tban [durasi]</code> - Ban sementara\n\n\
    <b>Cara Kerja:</b>\n\
    Jika user mengirim lebih dari X pesan dalam waktu singkat, hukuman diterapkan.\n\n\
    <b>Bypass:</b>\n\
    Admin dan user yang di-approve tidak terkena antiflood."
    .to_string()
}

pub fn get_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback("🔙 Kembali", "help:back")],
    ])
}
