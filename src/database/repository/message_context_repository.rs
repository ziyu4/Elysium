//! MessageContext repository with hot caching.
//!
//! Stores antiflood config + approved users for per-message checks.
//! Aggressively cached with 10min TTL.

use std::time::Duration;

use anyhow::Result;
use mongodb::bson::doc;
use mongodb::Collection;
use tracing::debug;

use crate::cache::{CacheConfig, CacheRegistry, TypedCache};
use crate::database::models::MessageContext;
use crate::database::Database;

/// Repository for message context (antiflood + approved users).
pub struct MessageContextRepository {
    collection: Collection<MessageContext>,
    cache: TypedCache<i64, MessageContext>,
}

impl MessageContextRepository {
    pub fn new(db: &Database, cache: &CacheRegistry) -> Self {
        let context_cache = cache.get_or_create(
            "message_context",
            CacheConfig::with_capacity(10_000)
                .ttl(Duration::from_secs(600)), // 10 minutes
        );

        Self {
            collection: db.collection("message_context"),
            cache: context_cache,
        }
    }

    /// Get context, returning default if not exists.
    pub async fn get_or_default(&self, chat_id: i64) -> Result<MessageContext> {
        if let Some(ctx) = self.cache.get(&chat_id) {
            return Ok(ctx);
        }

        let filter = doc! { "chat_id": chat_id };
        let result = self.collection.find_one(filter).await?;

        let ctx = result.unwrap_or_else(|| MessageContext::new(chat_id));
        self.cache.insert(chat_id, ctx.clone());

        Ok(ctx)
    }

    /// Get context only if it exists.
    pub async fn get(&self, chat_id: i64) -> Result<Option<MessageContext>> {
        if let Some(ctx) = self.cache.get(&chat_id) {
            return Ok(Some(ctx));
        }

        let filter = doc! { "chat_id": chat_id };
        let result = self.collection.find_one(filter).await?;

        if let Some(ctx) = &result {
            self.cache.insert(chat_id, ctx.clone());
        }

        Ok(result)
    }

    /// Save context (upsert).
    pub async fn save(&self, ctx: &MessageContext) -> Result<()> {
        let filter = doc! { "chat_id": ctx.chat_id };
        let options = mongodb::options::ReplaceOptions::builder()
            .upsert(true)
            .build();

        self.collection
            .replace_one(filter, ctx)
            .with_options(options)
            .await?;

        self.cache.insert(ctx.chat_id, ctx.clone());
        debug!("Saved MessageContext for chat {}", ctx.chat_id);

        Ok(())
    }

    /// Update antiflood config.
    pub async fn update_antiflood(
        &self,
        chat_id: i64,
        antiflood: crate::database::models::AntifloodConfig,
    ) -> Result<()> {
        let mut ctx = self.get_or_default(chat_id).await?;
        ctx.antiflood = antiflood;
        self.save(&ctx).await
    }

    /// Approve a user.
    pub async fn approve_user(&self, chat_id: i64, user_id: u64) -> Result<bool> {
        let mut ctx = self.get_or_default(chat_id).await?;
        let approved = ctx.approve_user(user_id);
        if approved {
            self.save(&ctx).await?;
        }
        Ok(approved)
    }

    /// Unapprove a user.
    pub async fn unapprove_user(&self, chat_id: i64, user_id: u64) -> Result<bool> {
        let mut ctx = self.get_or_default(chat_id).await?;
        let removed = ctx.unapprove_user(user_id);
        if removed {
            self.save(&ctx).await?;
        }
        Ok(removed)
    }

    /// Unapprove all users.
    pub async fn unapprove_all(&self, chat_id: i64) -> Result<usize> {
        let mut ctx = self.get_or_default(chat_id).await?;
        let count = ctx.unapprove_all();
        if count > 0 {
            self.save(&ctx).await?;
        }
        Ok(count)
    }

    /// Invalidate cache for a chat.
    pub fn invalidate(&self, chat_id: i64) {
        self.cache.invalidate(&chat_id);
    }
}
