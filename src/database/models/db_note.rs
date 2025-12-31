//! Note model for distinct collection.

use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

use crate::database::InlineButton;

/// A single note document (stored in `notes` collection).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbNote {
    /// MongoDB document ID
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,

    /// Chat ID this note belongs to
    pub chat_id: i64,

    /// Note name (indexed)
    pub name: String,

    /// Note content
    pub content: String,

    /// Buttons for the note
    #[serde(default)]
    pub buttons: Vec<Vec<InlineButton>>,

    /// Media file ID (if any)
    #[serde(default)]
    pub file_id: Option<String>,

    /// Media type
    #[serde(default)]
    pub file_type: Option<String>,

    /// Protect content
    #[serde(default)]
    pub protect: bool,

    /// Admin only view
    #[serde(default)]
    pub admin_only: bool,
}

impl DbNote {
    /// Create a new note.
    pub fn new(chat_id: i64, name: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: None,
            chat_id,
            name: name.into(),
            content: content.into(),
            buttons: vec![],
            file_id: None,
            file_type: None,
            protect: false,
            admin_only: false,
        }
    }
}
