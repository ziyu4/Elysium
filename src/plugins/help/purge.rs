use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub fn get_text() -> String {
    "<b>🗑️ Bantuan: Purge</b>\n\n\
    Hapus banyak pesan sekaligus.\n\n\
    <b>Perintah:</b>\n\
    • <code>/purge</code> - Hapus pesan dari reply sampai sekarang\n\
    • <code>/spurge</code> - Silent purge (tanpa konfirmasi)\n\
    • <code>/del</code> - Hapus 1 pesan yang di-reply\n\
    • <code>/purgefrom</code> - Tandai titik awal\n\
    • <code>/purgeto</code> - Hapus dari purgefrom sampai sini\n\n\
    <b>Cara Pakai:</b>\n\
    1. Reply ke pesan awal\n\
    2. Ketik <code>/purge</code>\n\
    3. Semua pesan dari reply sampai command akan dihapus\n\n\
    <b>Catatan:</b>\n\
    Bot hanya bisa hapus pesan < 48 jam."
    .to_string()
}

pub fn get_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback("🔙 Kembali", "help:back")],
    ])
}
