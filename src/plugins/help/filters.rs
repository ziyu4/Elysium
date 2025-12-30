use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub fn get_text() -> String {
    "<b>⚙️ Bantuan: Filters</b>\n\n\
    Filter memungkinkan bot membalas otomatis ketika kata kunci tertentu terdeteksi.\n\n\
    <b>Perintah:</b>\n\
    • <code>/filter &lt;trigger&gt; &lt;balasan&gt;</code> - Tambah filter\n\
    • <code>/stop &lt;trigger&gt;</code> - Hapus filter\n\
    • <code>/filters</code> - Lihat daftar filter\n\
    • <code>/stopall</code> - Hapus semua filter\n\n\
    <b>Tipe Trigger:</b>\n\
    • <code>kata</code> - Match di mana saja (default)\n\
    • <code>=kata</code> - Exact match (pesan = trigger)\n\
    • <code>*kata</code> - Prefix match (pesan dimulai dengan)\n\n\
    <b>Multi-Trigger:</b>\n\
    <code>/filter (hi, halo, hey) Halo juga!</code>\n\n\
    <b>Format Tombol:</b>\n\
    <code>/filter test Coba ini! {button:Klik|https://...}</code>\n\n\
    <b>Permission Tags:</b>\n\
    • <code>{admin}</code> - Hanya admin bisa trigger\n\
    • <code>{user}</code> - Hanya non-admin\n\
    • <code>{protect}</code> - Konten tidak bisa di-forward\n\
    • <code>{replytag}</code> - Reply ke user yang di-reply\n\n\
    <b>Contoh:</b>\n\
    <code>/filter rules Baca peraturan! {button:Rules|https://t.me/...}</code>"
    .to_string()
}

pub fn get_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback("🔙 Kembali", "help:back")],
    ])
}
