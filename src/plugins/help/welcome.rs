use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub fn get_text() -> String {
    "<b>👋 Bantuan: Welcome</b>\n\n\
    Fitur welcome mengirim pesan otomatis saat member baru bergabung.\n\n\
    <b>Perintah:</b>\n\
    • <code>/welcome</code> - Lihat status & preview\n\
    • <code>/welcome on/off</code> - Aktifkan/nonaktifkan\n\
    • <code>/setwelcome</code> - Atur pesan (reply ke pesan)\n\
    • <code>/setwelcomebuttons</code> - Atur tombol\n\
    • <code>/resetwelcome</code> - Reset ke default\n\n\
    <b>Format Tombol:</b>\n\
    • <code>{button:Teks|URL}</code> - Satu tombol\n\
    • <code>{button:A|URL}:{button:B|URL}</code> - Satu baris (pakai :)\n\
    • Baris baru = baris tombol baru\n\n\
    <b>Placeholder:</b>\n\
    • <code>{first}</code> - Nama depan\n\
    • <code>{last}</code> - Nama belakang\n\
    • <code>{fullname}</code> - Nama lengkap\n\
    • <code>{mention}</code> - Mention user\n\
    • <code>{id}</code> - User ID\n\
    • <code>{group}</code> - Nama grup\n\
    • <code>{count}</code> - Jumlah member\n\n\
    <b>Contoh:</b>\n\
    <code>/setwelcome Selamat datang {mention} di {group}! {button:Rules|https://t.me/...}</code>"
    .to_string()
}

pub fn get_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback("🔙 Kembali", "help:back")],
    ])
}
