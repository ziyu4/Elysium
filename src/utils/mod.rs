//! Utility functions.
//!
//! Collection of helper functions used across the bot.

pub mod parser;

pub use parser::{
    parse_content, apply_fillings as apply_fillings_new, html_escape, format_duration_full, parse_duration,
};

/// Format a username for display.
///
/// If the user has a username, returns @username.
/// Otherwise, returns the first name.
#[allow(dead_code)]
pub fn format_username(username: Option<&str>, first_name: &str) -> String {
    match username {
        Some(u) => format!("@{}", u),
        None => first_name.to_string(),
    }
}

/// Escape special characters for MarkdownV2.
#[allow(dead_code)]
pub fn escape_markdown(text: &str) -> String {
    let special_chars = [
        '_', '*', '[', ']', '(', ')', '~', '`', '>', '#', '+', '-', '=', '|', '{', '}', '.', '!',
    ];

    let mut result = String::with_capacity(text.len() * 2);
    for c in text.chars() {
        if special_chars.contains(&c) {
            result.push('\\');
        }
        result.push(c);
    }
    result
}
