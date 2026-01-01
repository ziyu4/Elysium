//! Message dispatcher setup.
//!
//! Builds the dispatcher with all command handlers and event handlers.

use std::sync::Arc;

use teloxide::adaptors::Throttle;
use teloxide::dispatching::UpdateHandler;
use teloxide::prelude::*;

use crate::cache::CacheRegistry;
use crate::database::{
    Database, UserRepo, FilterRepository, NoteRepository,
    MessageContextRepository, WelcomeRepository, ByeRepository,
    RulesRepository, WarnsRepository,
};
use crate::events::{self, FloodTracker};
use crate::permissions::Permissions;
use crate::plugins;

/// Bot type with Throttle adaptor for automatic rate limiting.
pub type ThrottledBot = Throttle<Bot>;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    /// Database connection.
    pub db: Arc<Database>,

    /// Cache registry for creating/accessing caches.
    pub cache: Arc<CacheRegistry>,

    /// Permission checker with admin caching.
    pub permissions: Permissions,

    /// User repository for tracking and resolving users.
    pub users: Arc<UserRepo>,

    /// Filter repository.
    pub filters: Arc<FilterRepository>,

    /// Note repository.
    pub notes: Arc<NoteRepository>,
    
    /// Message context repository (antiflood + approved users).
    pub message_context: Arc<MessageContextRepository>,

    /// Welcome repository.
    pub welcome: Arc<WelcomeRepository>,

    /// Bye repository.
    pub bye: Arc<ByeRepository>,

    /// Rules repository.
    pub rules: Arc<RulesRepository>,

    /// Warns repository.
    pub warns: Arc<WarnsRepository>,

    /// Owner user IDs (bypass all restrictions).
    pub owner_ids: Vec<u64>,

    /// Bot username (without @) for deep link construction.
    pub bot_username: String,
}

impl AppState {
    /// Create a new application state.
    pub fn new(
        bot: ThrottledBot,
        db: Arc<Database>,
        cache: Arc<CacheRegistry>,
        owner_ids: Vec<u64>,
        bot_username: String,
    ) -> Self {
        // Note: Permissions needs the inner Bot for API calls
        let permissions = Permissions::with_owners(bot.inner().clone(), cache.clone(), owner_ids.clone());

        // Create repositories
        let users = Arc::new(UserRepo::new(&db, &cache));
        let filters = Arc::new(FilterRepository::new(&db, &cache));
        let notes = Arc::new(NoteRepository::new(&db, &cache));
        let message_context = Arc::new(MessageContextRepository::new(&db, &cache));
        let welcome = Arc::new(WelcomeRepository::new(&db, &cache));
        let bye = Arc::new(ByeRepository::new(&db, &cache));
        let rules = Arc::new(RulesRepository::new(&db, &cache));
        let warns = Arc::new(WarnsRepository::new(&db, &cache));

        Self {
            db,
            cache,
            permissions,
            users,
            filters,
            notes,
            message_context,
            welcome,
            bye,
            rules,
            warns,
            owner_ids,
            bot_username,
        }
    }

    /// Check if a user is a bot owner.
    pub fn is_owner(&self, user_id: u64) -> bool {
        self.owner_ids.contains(&user_id)
    }

    /// Resolve locale for a context (User + Chat).
    pub async fn get_locale(&self, chat_id: Option<i64>, user_id: Option<u64>) -> String {
        let mut group_lang = None;
        let mut user_lang = None;

        // check group lang
        if let Some(chat) = chat_id {
             if let Ok(ctx) = self.message_context.get_or_default(chat).await {
                 if let Some(info) = ctx.group_info {
                     group_lang = info.lang;
                 }
                 // If not set, maybe check antiflood config or other settings? Nah.
             }
        }

        // check user lang
        if let Some(uid) = user_id {
            if let Ok(Some(u)) = self.users.get_by_id(uid).await {
                user_lang = u.lang;
            }
        }
        
        crate::i18n::resolve_locale(group_lang.as_deref(), user_lang.as_deref())
    }
}

/// Build the dispatcher with all handlers.
pub fn build_dispatcher(
    bot: ThrottledBot,
    db: Arc<Database>,
    cache: Arc<CacheRegistry>,
    owner_ids: Vec<u64>,
    bot_username: String,
) -> Dispatcher<ThrottledBot, anyhow::Error, teloxide::dispatching::DefaultKey> {
    let state = AppState::new(bot.clone(), db, cache, owner_ids, bot_username);
    let flood_tracker = FloodTracker::new();

    Dispatcher::builder(bot, schema())
        .dependencies(dptree::deps![state, flood_tracker])
        .enable_ctrlc_handler()
        .build()
}

/// Build the handler schema.
fn schema() -> UpdateHandler<anyhow::Error> {
    use teloxide::dispatching::UpdateFilterExt;

    // Message handlers: user tracking first, then commands, hashtags, events
    let message_handler = Update::filter_message()
        .inspect_async(track_user)
        .branch(plugins::command_handler())
        .branch(plugins::hashtag_handler())
        .branch(events::message_event_handler());

    // Chat member events (welcome new members)
    let member_handler = Update::filter_chat_member()
        .branch(events::event_handler());

    // Callback query handler
    let callback_handler = plugins::callback_handler();

    dptree::entry()
        .branch(message_handler)
        .branch(member_handler)
        .branch(callback_handler)
}

/// Track user from message (runs before all handlers).
async fn track_user(msg: Message, state: AppState) {
    if let Some(user) = msg.from.as_ref() {
        state.users.clone().upsert_background(user.clone());
    }
}
