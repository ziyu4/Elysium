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
            owner_ids,
            bot_username,
        }
    }

    /// Check if a user is a bot owner.
    pub fn is_owner(&self, user_id: u64) -> bool {
        self.owner_ids.contains(&user_id)
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
