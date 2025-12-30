//! Notes configuration models.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::common::InlineButton;

/// Tags that modify note behavior.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NoteTags {
    /// Only admins can view this note
    #[serde(default)]
    pub admin_only: bool,

    /// Always send to PM
    #[serde(default)]
    pub is_private: bool,

    /// Never send to PM (override global setting)
    #[serde(default)]
    pub no_private: bool,

    /// Protect from forwarding
    #[serde(default)]
    pub protect: bool,

    /// Enable link preview
    #[serde(default)]
    pub preview: bool,

    /// Send without notification
    #[serde(default)]
    pub no_notif: bool,

    /// Mark media as spoiler
    #[serde(default)]
    pub media_spoiler: bool,
}

/// A saved note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    /// Note name (unique per chat, lowercase)
    pub name: String,

    /// Note text content (with fillings/placeholders)
    #[serde(default)]
    pub text: Option<String>,

    /// Media file_id (reference to cached media)
    #[serde(default)]
    pub media_file_id: Option<String>,

    /// Media type: "photo", "video", "animation", "sticker", "document"
    #[serde(default)]
    pub media_type: Option<String>,

    /// Inline buttons (parsed from text)
    #[serde(default)]
    pub buttons: Vec<Vec<InlineButton>>,

    /// Behavior tags (parsed from text)
    #[serde(default)]
    pub tags: NoteTags,
}

impl Note {
    /// Create a new note with just a name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into().to_lowercase(),
            text: None,
            media_file_id: None,
            media_type: None,
            buttons: vec![],
            tags: NoteTags::default(),
        }
    }
}

/// Notes configuration for a group.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NotesConfig {
    /// Whether to send notes via PM by default
    #[serde(default)]
    pub private_notes: bool,

    /// All notes stored by name (lowercase)
    #[serde(default)]
    pub notes: HashMap<String, Note>,
}

impl NotesConfig {
    /// Get a note by name.
    pub fn get(&self, name: &str) -> Option<&Note> {
        self.notes.get(&name.to_lowercase())
    }

    /// Save a note.
    pub fn save(&mut self, note: Note) {
        self.notes.insert(note.name.clone(), note);
    }

    /// Delete a note by name.
    pub fn delete(&mut self, name: &str) -> Option<Note> {
        self.notes.remove(&name.to_lowercase())
    }

    /// Clear all notes.
    pub fn clear_all(&mut self) -> usize {
        let count = self.notes.len();
        self.notes.clear();
        count
    }

    /// List all note names.
    pub fn list_names(&self) -> Vec<&str> {
        self.notes.keys().map(|s| s.as_str()).collect()
    }
}
