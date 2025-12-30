use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub fn get_text() -> String {
    "<b>⚠️ Bantuan: Warns</b>\n\n\
    Sistem peringatan untuk mengelola pelanggaran user.\n\n\
    <b>Perintah Dasar:</b>\n\
    • <code>/warn [alasan]</code> - Beri peringatan\n\
    • <code>/dwarn</code> - Warn + hapus pesan (reply)\n\
    • <code>/swarn</code> - Silent warn\n\
    • <code>/warns [@user]</code> - Lihat peringatan user\n\
    • <code>/rmwarn</code> - Hapus peringatan terakhir\n\
    • <code>/resetwarn</code> - Reset semua peringatan user\n\
    • <code>/resetallwarns</code> - Reset SEMUA peringatan grup\n\n\
    <b>Pengaturan:</b>\n\
    • <code>/warnings</code> - Lihat konfigurasi\n\
    • <code>/warnmode [mode]</code> - Ubah mode hukuman\n\
    • <code>/warnlimit [angka]</code> - Ubah batas peringatan\n\
    • <code>/warntime [durasi]</code> - Durasi berlaku warn\n\n\
    <b>Mode Hukuman:</b>\n\
    • <code>ban</code> - Ban permanen\n\
    • <code>mute</code> - Mute permanen\n\
    • <code>kick</code> - Kick dari grup\n\
    • <code>tban [durasi]</code> - Ban sementara\n\
    • <code>tmute [durasi]</code> - Mute sementara\n\n\
    <b>Target:</b>\n\
    Reply ke pesan, atau gunakan @username / ID"
    .to_string()
}

pub fn get_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback("🔙 Kembali", "help:back")],
    ])
}
