use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub fn get_text() -> String {
    "<b>✅ Bantuan: Approval</b>\n\n\
    Sistem approval untuk bypass antiflood.\n\n\
    <b>Perintah:</b>\n\
    • <code>/approve [@user]</code> - Approve user\n\
    • <code>/unapprove [@user]</code> - Hapus approval\n\
    • <code>/approved</code> - Lihat daftar user approved\n\
    • <code>/approval</code> - Cek status approval Anda\n\
    • <code>/unapproveall</code> - Hapus semua approval\n\n\
    <b>Keuntungan Approved:</b>\n\
    • Tidak terkena antiflood\n\
    • Bebas dari batasan spam\n\n\
    <b>Target:</b>\n\
    Reply ke pesan, atau gunakan @username / ID"
    .to_string()
}

pub fn get_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback("🔙 Kembali", "help:back")],
    ])
}
