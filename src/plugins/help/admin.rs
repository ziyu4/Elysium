use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub fn get_text() -> String {
    "<b>🛡️ Bantuan: Admin</b>\n\n\
    Perintah khusus untuk administrator grup.\n\n\
    <b>📚 User Commands:</b>\n\
    • <code>/kickme</code> - Kick diri sendiri dari grup\n\n\
    <b>🚫 Ban Commands:</b>\n\
    • <code>/ban</code> - Ban user\n\
    • <code>/dban</code> - Delete pesan &amp; ban (reply)\n\
    • <code>/sban</code> - Silent ban (hapus perintah, tanpa pesan)\n\
    • <code>/tban &lt;waktu&gt;</code> - Ban sementara (4m, 3h, 6d, 5w)\n\
    • <code>/unban</code> - Unban user\n\n\
    <b>🔇 Mute Commands:</b>\n\
    • <code>/mute [waktu]</code> - Mute user (opsional durasi)\n\
    • <code>/dmute</code> - Delete pesan &amp; mute (reply)\n\
    • <code>/smute</code> - Silent mute (hapus perintah)\n\
    • <code>/tmute &lt;waktu&gt;</code> - Mute sementara\n\
    • <code>/unmute</code> - Unmute user\n\n\
    <b>👢 Kick Commands:</b>\n\
    • <code>/kick</code> - Kick user\n\
    • <code>/dkick</code> - Delete pesan &amp; kick (reply)\n\
    • <code>/skick</code> - Silent kick\n\n\
    <b>📌 Pin Commands:</b>\n\
    • <code>/pinned</code> - Lihat pesan yang dipin\n\
    • <code>/pin [loud]</code> - Pin pesan (tambah loud untuk notifikasi)\n\
    • <code>/permapin &lt;teks&gt;</code> - Pin pesan custom\n\
    • <code>/unpin</code> - Unpin pesan\n\
    • <code>/unpinall</code> - Unpin semua pesan\n\n\
    <b>👑 Admin Commands:</b>\n\
    • <code>/promote</code> - Jadikan admin (reply)\n\
    • <code>/demote</code> - Hapus admin (reply)\n\n\
    <b>📝 Contoh:</b>\n\
    • Mute @username selama 2 jam:\n  → <code>/tmute @username 2h</code>\n\
    • Silent ban ID 1234:\n  → <code>/sban 1234</code>\n\
    • Mute dengan durasi:\n  → <code>/mute @username 30m</code>"
    .to_string()
}

pub fn get_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback("🔙 Kembali", "help:back")],
    ])
}
