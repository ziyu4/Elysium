//! Plugin system for command handlers.
//!
//! Add new plugins by:
//! 1. Creating a new file in this directory
//! 2. Adding `pub mod your_plugin;` below
//! 3. Adding the handler to `command_handler()`

pub mod admin;
pub mod afk;
pub mod antiflood;
pub mod approval;
pub mod ban;
pub mod bye;
pub mod filters;
pub mod help;
pub mod mute;
pub mod notes;
pub mod pin;
pub mod purge;
pub mod rules;
pub mod start;
pub mod warn;
pub mod welcome;

use teloxide::dispatching::UpdateHandler;
use teloxide::prelude::*;
use teloxide::types::Me;
use teloxide::utils::command::BotCommands;

use crate::bot::dispatcher::{AppState, ThrottledBot};

/// All bot commands.
#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Perintah yang tersedia:")]
pub enum Command {
    #[command(description = "Mulai bot")]
    Start(String),

    #[command(description = "Bantuan")]
    Help,

    // Antiflood commands
    #[command(description = "Pengaturan antiflood")]
    Antiflood,

    #[command(description = "Atur limit flood")]
    Setflood,

    #[command(description = "Atur hukuman flood")]
    Setfloodpenalty,

    // Approval commands
    #[command(description = "Approve user (bypass antiflood)")]
    Approve,

    #[command(description = "Unapprove user")]
    Unapprove,

    #[command(description = "Unapprove semua user")]
    Unapproveall,

    #[command(description = "Cek status approval Anda")]
    Approval,

    #[command(description = "Lihat daftar user approved")]
    Approved,

    // Notes commands
    #[command(description = "Simpan note")]
    Save,

    #[command(description = "Ambil note")]
    Get,

    #[command(description = "Lihat semua notes")]
    Notes,

    #[command(description = "Lihat semua notes")]
    Saved,

    #[command(description = "Hapus note")]
    Clear,

    #[command(description = "Hapus semua notes")]
    Clearall,

    #[command(description = "Toggle notes ke PM")]
    Privatenotes,

    // Welcome commands
    #[command(description = "Pengaturan welcome")]
    Welcome,

    #[command(description = "Atur pesan welcome")]
    Setwelcome,

    #[command(description = "Atur tombol welcome")]
    Setwelcomebuttons,

    #[command(description = "Reset welcome")]
    Resetwelcome,

    // Rules commands
    #[command(description = "Lihat peraturan grup")]
    Rules,

    #[command(description = "Atur peraturan grup")]
    Setrules,

    #[command(description = "Hapus peraturan")]
    Clearrules,

    #[command(description = "Atur tampilan rules di PM/grup")]
    Setrulesprivate,

    // Admin commands
    #[command(description = "Promosikan user menjadi admin")]
    Promote,

    #[command(description = "Demote admin menjadi member")]
    Demote,

    // Filter commands
    #[command(description = "Tambah filter auto-reply")]
    Filter,

    #[command(description = "Lihat semua filter")]
    Filters,

    #[command(description = "Hapus filter")]
    Stop,

    #[command(description = "Hapus semua filter")]
    Stopall,

    // AFK commands
    #[command(description = "Set status AFK")]
    Afk,

    #[command(description = "Set status AFK (alias)")]
    Brb,

    // Pin commands
    #[command(description = "Pin pesan (reply)")]
    Pin,
    
    #[command(description = "Unpin pesan")]
    Unpin,
    
    #[command(description = "Pin pesan (silent)")]
    Permapin,

    // Ban commands
    #[command(description = "Ban user")]
    Ban,
    
    #[command(description = "Unban user")]
    Unban,
    
    #[command(description = "Kick user")]
    Kick,
    
    #[command(description = "Temp ban user")]
    Tban,
    
    #[command(description = "Delete pesan & ban user")]
    Dban,
    
    #[command(description = "Silent ban user")]
    Sban,
    
    #[command(description = "Delete pesan & kick user")]
    Dkick,
    
    #[command(description = "Silent kick user")]
    Skick,
    
    #[command(description = "Kick diri sendiri")]
    Kickme,

    // Mute commands
    #[command(description = "Mute user")]
    Mute,
    
    #[command(description = "Unmute user")]
    Unmute,
    
    #[command(description = "Temp mute user")]
    Tmute,
    
    #[command(description = "Delete pesan & mute user")]
    Dmute,
    
    #[command(description = "Silent mute user")]
    Smute,
    
    // Pin commands (additional)
    #[command(description = "Lihat pesan yang dipin")]
    Pinned,
    
    #[command(description = "Unpin semua pesan")]
    Unpinall,

    // Purge commands
    #[command(description = "Hapus pesan dari reply sampai sekarang")]
    Purge,
    
    #[command(description = "Silent purge")]
    Spurge,
    
    #[command(description = "Hapus replied message")]
    Del,
    
    #[command(description = "Tandai titik awal purge")]
    Purgefrom,
    
    #[command(description = "Hapus range dari purgefrom")]
    Purgeto,

    // Bye commands
    #[command(description = "Pengaturan goodbye")]
    Bye,
    
    #[command(description = "Atur pesan goodbye")]
    Setbye,
    
    #[command(description = "Atur tombol goodbye")]
    Setbyebuttons,
    
    #[command(description = "Reset goodbye")]
    Resetbye,

    // Warning commands
    #[command(description = "Beri peringatan user")]
    Warn,
    
    #[command(description = "Peringatan + hapus pesan")]
    Dwarn,
    
    #[command(description = "Peringatan diam-diam")]
    Swarn,
    
    #[command(description = "Lihat peringatan user")]
    Warns,
    
    #[command(description = "Hapus peringatan terakhir")]
    Rmwarn,
    
    #[command(description = "Reset semua peringatan user")]
    Resetwarn,
    
    #[command(description = "Reset SEMUA peringatan di grup")]
    Resetallwarns,
    
    #[command(description = "Lihat pengaturan peringatan")]
    Warnings,
    
    #[command(description = "Ubah mode peringatan")]
    Warnmode,
    
    #[command(description = "Ubah batas peringatan")]
    Warnlimit,
    
    #[command(description = "Ubah masa berlaku peringatan")]
    Warntime,
}

/// Build the combined command handler.
pub fn command_handler() -> UpdateHandler<anyhow::Error> {
    use dptree::case;

    

    teloxide::filter_command::<Command, _>()
        .branch(case![Command::Start(args)].endpoint(handle_start))
        .branch(case![Command::Help].endpoint(handle_help))
        // Antiflood
        .branch(case![Command::Antiflood].endpoint(antiflood::antiflood_command))
        .branch(case![Command::Setflood].endpoint(antiflood::setflood_command))
        .branch(case![Command::Setfloodpenalty].endpoint(antiflood::setfloodpenalty_command))
        // Approval
        .branch(case![Command::Approve].endpoint(approval::approve_command))
        .branch(case![Command::Unapprove].endpoint(approval::unapprove_command))
        .branch(case![Command::Unapproveall].endpoint(approval::unapproveall_command))
        .branch(case![Command::Approval].endpoint(approval::approval_command))
        .branch(case![Command::Approved].endpoint(approval::approved_command))
        // Notes
        .branch(case![Command::Save].endpoint(notes::save_command))
        .branch(case![Command::Get].endpoint(handle_get))
        .branch(case![Command::Notes].endpoint(notes::notes_command))
        .branch(case![Command::Saved].endpoint(notes::notes_command))
        .branch(case![Command::Clear].endpoint(notes::clear_command))
        .branch(case![Command::Clearall].endpoint(notes::clearall_command))
        .branch(case![Command::Privatenotes].endpoint(notes::privatenotes_command))
        // Welcome
        .branch(case![Command::Welcome].endpoint(welcome::welcome_command))
        .branch(case![Command::Setwelcome].endpoint(welcome::setwelcome_command))
        .branch(case![Command::Setwelcomebuttons].endpoint(welcome::setwelcomebuttons_command))
        .branch(case![Command::Resetwelcome].endpoint(welcome::resetwelcome_command))
        // Rules
        .branch(case![Command::Rules].endpoint(handle_rules))
        .branch(case![Command::Setrules].endpoint(rules::setrules_command))
        .branch(case![Command::Clearrules].endpoint(rules::clearrules_command))
        .branch(case![Command::Setrulesprivate].endpoint(rules::setrulesprivate_command))
        // Admin
        .branch(case![Command::Promote].endpoint(admin::promote_command))
        .branch(case![Command::Demote].endpoint(admin::demote_command))
        // Filters
        .branch(case![Command::Filter].endpoint(filters::filter_command))
        .branch(case![Command::Filters].endpoint(filters::filters_command))
        .branch(case![Command::Stop].endpoint(filters::stop_command))
        .branch(case![Command::Stopall].endpoint(filters::stopall_command))
        // AFK
        .branch(case![Command::Afk].endpoint(afk::afk_command))
        .branch(case![Command::Brb].endpoint(afk::brb_command))
        // Pin
        .branch(case![Command::Pin].endpoint(pin::pin_command))
        .branch(case![Command::Unpin].endpoint(pin::unpin_command))
        .branch(case![Command::Permapin].endpoint(pin::permapin_command))
        // Ban
        .branch(case![Command::Ban].endpoint(ban::ban_command))
        .branch(case![Command::Unban].endpoint(ban::unban_command))
        .branch(case![Command::Kick].endpoint(ban::kick_command))
        .branch(case![Command::Tban].endpoint(ban::tban_command))
        .branch(case![Command::Dban].endpoint(ban::dban_command))
        .branch(case![Command::Sban].endpoint(ban::sban_command))
        .branch(case![Command::Dkick].endpoint(ban::dkick_command))
        .branch(case![Command::Skick].endpoint(ban::skick_command))
        .branch(case![Command::Kickme].endpoint(ban::kickme_command))
        // Mute
        .branch(case![Command::Mute].endpoint(mute::mute_command))
        .branch(case![Command::Unmute].endpoint(mute::unmute_command))
        .branch(case![Command::Tmute].endpoint(mute::tmute_command))
        .branch(case![Command::Dmute].endpoint(mute::dmute_command))
        .branch(case![Command::Smute].endpoint(mute::smute_command))
        // Pin (additional)
        .branch(case![Command::Pinned].endpoint(pin::pinned_command))
        .branch(case![Command::Unpinall].endpoint(pin::unpinall_command))
        // Purge
        .branch(case![Command::Purge].endpoint(purge::purge_command))
        .branch(case![Command::Spurge].endpoint(purge::spurge_command))
        .branch(case![Command::Del].endpoint(purge::del_command))
        .branch(case![Command::Purgefrom].endpoint(purge::purgefrom_command))
        .branch(case![Command::Purgeto].endpoint(purge::purgeto_command))
        // Bye
        .branch(case![Command::Bye].endpoint(bye::bye_command))
        .branch(case![Command::Setbye].endpoint(bye::setbye_command))
        .branch(case![Command::Setbyebuttons].endpoint(bye::setbyebuttons_command))
        .branch(case![Command::Resetbye].endpoint(bye::resetbye_command))
        // Warning
        .branch(case![Command::Warn].endpoint(warn::warn_command))
        .branch(case![Command::Dwarn].endpoint(warn::dwarn_command))
        .branch(case![Command::Swarn].endpoint(warn::swarn_command))
        .branch(case![Command::Warns].endpoint(warn::warns_command))
        .branch(case![Command::Rmwarn].endpoint(warn::rmwarn_command))
        .branch(case![Command::Resetwarn].endpoint(warn::resetwarn_command))
        .branch(case![Command::Resetallwarns].endpoint(warn::resetallwarns_command))
        .branch(case![Command::Warnings].endpoint(warn::warnings_command))
        .branch(case![Command::Warnmode].endpoint(warn::warnmode_command))
        .branch(case![Command::Warnlimit].endpoint(warn::warnlimit_command))
        .branch(case![Command::Warntime].endpoint(warn::warntime_command))
}

/// Build hashtag handler for notes.
pub fn hashtag_handler() -> UpdateHandler<anyhow::Error> {
    dptree::filter(|msg: Message| {
        msg.text()
            .map(|t| t.starts_with('#') && !t.starts_with("##"))
            .unwrap_or(false)
    })
    .endpoint(handle_hashtag)
}

/// Handle hashtag note shortcut (uses state.bot_username).
async fn handle_hashtag(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    notes::handle_hashtag_note(bot, msg, state).await
}

/// Handle /start command with optional deep link.
async fn handle_start(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
    args: String,
) -> anyhow::Result<()> {
    // Check for deep links
    if args.starts_with("rules_") {
        let chat_id_str = args.strip_prefix("rules_").unwrap();
        return rules::handle_rules_deeplink(bot, msg, state, chat_id_str).await;
    }

    // Help deep link
    if args == "help" {
        return help::send_help_menu(&bot, msg.chat.id).await;
    }

    // Default start message
    start::start_handler(bot, msg, state).await
}

/// Handle /help command.
async fn handle_help(bot: ThrottledBot, msg: Message, state: AppState) -> anyhow::Result<()> {
    help::help_handler(bot, msg, state).await
}

/// Build the callback query handler.
pub fn callback_handler() -> UpdateHandler<anyhow::Error> {
    Update::filter_callback_query()
        .branch(dptree::filter(|q: CallbackQuery| {
            q.data.as_ref().map(|d| d.starts_with("warn_remove:")).unwrap_or(false)
        }).endpoint(warn::warn_callback_handler))
        .branch(dptree::endpoint(help::callback_handler))
}

/// Handle /get command (uses state.bot_username).
async fn handle_get(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    notes::get_command(bot, msg, state).await
}

/// Handle /rules command (now uses state.bot_username).
async fn handle_rules(
    bot: ThrottledBot,
    msg: Message,
    state: AppState,
    _me: Me,
) -> anyhow::Result<()> {
    rules::rules_command(bot, msg, state).await
}
