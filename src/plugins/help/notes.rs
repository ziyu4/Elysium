use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub fn get_text() -> String {
    "<b>📝 Bantuan: Notes</b>\n\n\
    Fitur notes memungkinkan Anda menyimpan pesan, media, dan tombol dengan shortcut nama.\n\n\
    <b>Perintah:</b>\n\
    • <code>/save &lt;nama&gt; &lt;konten&gt;</code> - Simpan note baru\n\
    • <code>/get &lt;nama&gt;</code> - Tampilkan note (atau gunakan #nama)\n\
    • <code>/notes</code> - Lihat daftar semua notes\n\
    • <code>/clear &lt;nama&gt;</code> - Hapus note\n\
    • <code>/clearall</code> - Hapus semua notes (admin only)\n\
    • <code>/privatenotes on/off</code> - Kirim note ke PM\n\n\
    <b>Format Tombol:</b>\n\
    • <code>{button:Teks|URL}</code> - Satu tombol\n\
    • <code>{button:A|URL}:{button:B|URL}</code> - Satu baris\n\
    • Baris baru = baris tombol baru\n\n\
    <b>Contoh:</b>\n\
    <code>/save rules Baca peraturan! {button:Rules|https://t.me/...}</code>\n\n\
    <b>Permission Tags:</b>\n\
    • <code>{admin}</code> - Hanya admin bisa lihat\n\
    • <code>{user}</code> - Hanya non-admin\n\n\
    <b>Tips:</b>\n\
    • Gunakan <code>#nama</code> untuk memanggil note dengan cepat\n\
    • Reply ke user saat memanggil note untuk men-tag mereka"
    .to_string()
}

pub fn get_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback("🔙 Kembali", "help:back")],
    ])
}
