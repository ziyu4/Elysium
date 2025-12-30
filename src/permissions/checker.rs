//! Permission checker with caching.

use std::sync::Arc;
use std::time::Duration;

use teloxide::prelude::*;
use teloxide::types::{ChatId, ChatMember, ChatMemberKind, UserId};
use tracing::debug;

use crate::cache::{CacheConfig, CacheRegistry, TypedCache};

/// Cached admin information.
#[derive(Clone, Debug)]
pub struct AdminInfo {
    #[allow(dead_code)]
    pub user_id: UserId,
    pub is_owner: bool,
    pub can_delete_messages: bool,
    pub can_restrict_members: bool,
    pub can_promote_members: bool,
    pub can_change_info: bool,
    #[allow(dead_code)]
    pub can_invite_users: bool,
    pub can_pin_messages: bool,
    #[allow(dead_code)]
    pub can_manage_chat: bool,
}

impl AdminInfo {
    /// Create AdminInfo from a ChatMember.
    fn from_chat_member(member: &ChatMember) -> Option<Self> {
        match &member.kind {
            ChatMemberKind::Owner(_) => Some(Self {
                user_id: member.user.id,
                is_owner: true,
                can_delete_messages: true,
                can_restrict_members: true,
                can_promote_members: true,
                can_change_info: true,
                can_invite_users: true,
                can_pin_messages: true,
                can_manage_chat: true,
            }),
            ChatMemberKind::Administrator(admin) => Some(Self {
                user_id: member.user.id,
                is_owner: false,
                can_delete_messages: admin.can_delete_messages,
                can_restrict_members: admin.can_restrict_members,
                can_promote_members: admin.can_promote_members,
                can_change_info: admin.can_change_info,
                can_invite_users: admin.can_invite_users,
                can_pin_messages: admin.can_pin_messages,
                can_manage_chat: admin.can_manage_chat,
            }),
            _ => None,
        }
    }

    /// Create AdminInfo for a bot owner (has all permissions).
    fn bot_owner(user_id: UserId) -> Self {
        Self {
            user_id,
            is_owner: true, // Treated as owner for permission purposes
            can_delete_messages: true,
            can_restrict_members: true,
            can_promote_members: true,
            can_change_info: true,
            can_invite_users: true,
            can_pin_messages: true,
            can_manage_chat: true,
        }
    }
}

/// Cache key for admin lookups.
type AdminCacheKey = (i64, u64); // (chat_id, user_id)

/// Permission checker with caching support.
///
/// Bot owners (from OWNER_IDS env) automatically bypass all permission checks.
#[derive(Clone)]
pub struct Permissions {
    bot: Bot,
    cache: TypedCache<AdminCacheKey, Option<AdminInfo>>,
    /// Bot owner IDs - these users have all permissions in all chats.
    owner_ids: Vec<u64>,
}

impl Permissions {
    /// Create a new permission checker with bot owner IDs.
    ///
    /// Bot owners automatically have all permissions in all chats.
    pub fn with_owners(bot: Bot, cache_registry: Arc<CacheRegistry>, owner_ids: Vec<u64>) -> Self {
        let cache = cache_registry.get_or_create(
            "admin_permissions",
            CacheConfig::with_capacity(10_000)
                .ttl(Duration::from_secs(300)) // 5 minutes
                .tti(Duration::from_secs(120)), // 2 minutes idle
        );

        Self { bot, cache, owner_ids }
    }

    /// Check if a user is a bot owner.
    #[inline]
    pub fn is_bot_owner(&self, user_id: UserId) -> bool {
        self.owner_ids.contains(&user_id.0)
    }

    /// Get admin info for a user in a chat.
    ///
    /// Returns `None` if the user is not an admin.
    /// Bot owners always return Some with full permissions.
    pub async fn get_admin_info(
        &self,
        chat_id: ChatId,
        user_id: UserId,
    ) -> anyhow::Result<Option<AdminInfo>> {
        // Bot owners have all permissions
        if self.is_bot_owner(user_id) {
            debug!("User {} is bot owner, granting all permissions", user_id);
            return Ok(Some(AdminInfo::bot_owner(user_id)));
        }

        let cache_key = (chat_id.0, user_id.0);

        // Check cache first
        if let Some(cached) = self.cache.get(&cache_key) {
            debug!("Admin cache hit for user {} in chat {}", user_id, chat_id);
            return Ok(cached);
        }

        debug!("Admin cache miss for user {} in chat {}", user_id, chat_id);

        // Fetch from Telegram API
        let result = self.fetch_admin_info(chat_id, user_id).await?;

        // Cache the result (including None for non-admins)
        self.cache.insert(cache_key, result.clone());

        Ok(result)
    }

    /// Fetch admin info from Telegram API.
    async fn fetch_admin_info(
        &self,
        chat_id: ChatId,
        user_id: UserId,
    ) -> anyhow::Result<Option<AdminInfo>> {
        let member = self.bot.get_chat_member(chat_id, user_id).await?;
        Ok(AdminInfo::from_chat_member(&member))
    }

    /// Check if a user is an admin (including owner).
    /// Bot owners always return true.
    pub async fn is_admin(&self, chat_id: ChatId, user_id: UserId) -> anyhow::Result<bool> {
        if self.is_bot_owner(user_id) {
            return Ok(true);
        }
        Ok(self.get_admin_info(chat_id, user_id).await?.is_some())
    }

    /// Check if a user is the chat owner.
    /// Bot owners always return true.
    pub async fn is_owner(&self, chat_id: ChatId, user_id: UserId) -> anyhow::Result<bool> {
        if self.is_bot_owner(user_id) {
            return Ok(true);
        }
        Ok(self
            .get_admin_info(chat_id, user_id)
            .await?
            .map(|a| a.is_owner)
            .unwrap_or(false))
    }

    /// Check if a user can delete messages.
    /// Bot owners always return true.
    pub async fn can_delete_messages(
        &self,
        chat_id: ChatId,
        user_id: UserId,
    ) -> anyhow::Result<bool> {
        if self.is_bot_owner(user_id) {
            return Ok(true);
        }
        Ok(self
            .get_admin_info(chat_id, user_id)
            .await?
            .map(|a| a.can_delete_messages)
            .unwrap_or(false))
    }

    /// Check if a user can restrict members (ban, mute, etc.).
    /// Bot owners always return true.
    pub async fn can_restrict_members(
        &self,
        chat_id: ChatId,
        user_id: UserId,
    ) -> anyhow::Result<bool> {
        if self.is_bot_owner(user_id) {
            return Ok(true);
        }
        Ok(self
            .get_admin_info(chat_id, user_id)
            .await?
            .map(|a| a.can_restrict_members)
            .unwrap_or(false))
    }

    /// Check if a user can promote/demote admins.
    /// Bot owners always return true.
    pub async fn can_promote_members(
        &self,
        chat_id: ChatId,
        user_id: UserId,
    ) -> anyhow::Result<bool> {
        if self.is_bot_owner(user_id) {
            return Ok(true);
        }
        Ok(self
            .get_admin_info(chat_id, user_id)
            .await?
            .map(|a| a.can_promote_members)
            .unwrap_or(false))
    }

    /// Check if a user can change group info.
    /// Bot owners always return true.
    pub async fn can_change_info(
        &self,
        chat_id: ChatId,
        user_id: UserId,
    ) -> anyhow::Result<bool> {
        if self.is_bot_owner(user_id) {
            return Ok(true);
        }
        Ok(self
            .get_admin_info(chat_id, user_id)
            .await?
            .map(|a| a.can_change_info)
            .unwrap_or(false))
    }

    /// Check if a user can pin messages.
    /// Bot owners always return true.
    pub async fn can_pin_messages(
        &self,
        chat_id: ChatId,
        user_id: UserId,
    ) -> anyhow::Result<bool> {
        if self.is_bot_owner(user_id) {
            return Ok(true);
        }
        Ok(self
            .get_admin_info(chat_id, user_id)
            .await?
            .map(|a| a.can_pin_messages)
            .unwrap_or(false))
    }

    /// Invalidate cached admin info for a user.
    ///
    /// Call this when admin status might have changed.
    pub fn invalidate(&self, chat_id: ChatId, user_id: UserId) {
        let cache_key = (chat_id.0, user_id.0);
        self.cache.invalidate(&cache_key);
        debug!(
            "Invalidated admin cache for user {} in chat {}",
            user_id, chat_id
        );
    }

    /// Invalidate all cached admin info for a chat.
    ///
    /// Note: This is expensive, use sparingly.
    pub fn _invalidate_chat(&self, _chat_id: ChatId) {
        // Note: Moka doesn't support prefix invalidation easily
        // For now, we rely on TTL. In production, you might want
        // to track keys per chat for targeted invalidation.
        self.cache.invalidate_all();
        debug!("Invalidated all admin cache");
    }
}
