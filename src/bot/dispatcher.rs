//! Message dispatcher setup.
//!
//! Builds the dispatcher with all command handlers and event handlers.

use std::sync::Arc;

use teloxide::adaptors::Throttle;
use teloxide::dispatching::UpdateHandler;
use teloxide::prelude::*;

use crate::cache::CacheRegistry;
use crate::database::{Database, UserRepo};
use crate::events::{self, FloodTracker};
use crate::permissions::Permissions;
use crate::plugins;

/// Bot type with Throttle adaptor for automatic rate limiting.
pub type ThrottledBot = Throttle<Bot>;

/// Shared application state.
///
/// This state is available to all handlers via dependency injection.
/// Access it in handlers by adding `state: AppState` as a parameter.
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
        // The Throttle wrapper handles rate limiting automatically
        // Pass owner_ids so they can bypass all permission checks
        let permissions = Permissions::with_owners(bot.inner().clone(), cache.clone(), owner_ids.clone());

        // Create user repository
        let users = Arc::new(UserRepo::new(&db, &cache));

        Self {
            db,
            cache,
            permissions,
            users,
            owner_ids,
            bot_username,
        }
    }

    /// Check if a user is a bot owner (bypasses all restrictions).
    pub fn is_owner(&self, user_id: u64) -> bool {
        self.owner_ids.contains(&user_id)
    }
}

/// Build the dispatcher with all handlers.
///
/// This function creates and configures the dispatcher with:
/// - Command handlers (plugins)
/// - Event handlers (member updates, antiflood, etc.)
///
/// Note: The bot is wrapped with Throttle adaptor for automatic
/// rate limiting that respects Telegram's API limits.
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
///
/// The schema defines how updates are routed to handlers.
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
