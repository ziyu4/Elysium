//! Centralized content parser.
//!
//! This module provides unified parsing functionality for notes, filters,
//! welcome messages, and other content that supports:
//! - Buttons: `{button:Text|url}` syntax
//! - Tags: `{admin}`, `{user}`, `{protect}`, etc.
//! - Fillings: `{first}`, `{mention}`, `{chatname}`, etc.
//! - Random content: `%%%` separator

use teloxide::types::User;

use crate::database::InlineButton;

/// Content tags that modify behavior.
#[derive(Debug, Clone, Default)]
pub struct ContentTags {
    /// Only admins can trigger/see this content
    pub admin_only: bool,
    /// Only non-admins can trigger/see this content
    pub user_only: bool,
    /// Send to private/PM
    pub is_private: bool,
    /// Disable private sending
    pub no_private: bool,
    /// Protect content (no forward)
    pub protect: bool,
    /// Enable link preview
    pub preview: bool,
    /// Disable notification
    pub no_notif: bool,
    /// Media spoiler
    pub media_spoiler: bool,
    /// Reply to the user that was replied to (for filters)
    pub replytag: bool,
}

/// Result of parsing content.
#[derive(Debug, Clone)]
pub struct ParsedContent {
    /// Clean text without buttons/tags
    pub text: String,
    /// Extracted buttons (rows)
    pub buttons: Vec<Vec<InlineButton>>,
    /// Extracted tags
    pub tags: ContentTags,
    /// Random content parts (split by %%%)
    #[allow(dead_code)]
    pub random_parts: Vec<String>,
}

impl ParsedContent {
    /// Get a random part (or the text if no random parts)
    pub fn _get_random_text(&self) -> &str {
        if self.random_parts.is_empty() {
            &self.text
        } else if self.random_parts.len() == 1 {
            &self.random_parts[0]
        } else {
            // Simple pseudo-random using system time
            use std::time::{SystemTime, UNIX_EPOCH};
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0) as usize;
            let idx = nanos % self.random_parts.len();
            &self.random_parts[idx]
        }
    }
}

/// Parse content to extract buttons, tags, random parts, and clean text.
pub fn parse_content(input: &str) -> ParsedContent {
    // First, parse random parts (%%%)
    let random_parts = parse_random_parts(input);
    
    // If we have random parts, parse each one for buttons/tags
    // For simplicity, use the first part for structure, keep parts as-is
    let main_text = if random_parts.is_empty() {
        input.to_string()
    } else {
        // Parse buttons/tags from first part only for structure
        input.to_string()
    };
    
    let (text_without_buttons, buttons) = parse_buttons(&main_text);
    let (clean_text, tags) = parse_tags(&text_without_buttons);
    
    // Parse random from clean text if not already split
    let final_random_parts = if random_parts.len() > 1 {
        random_parts
    } else {
        vec![]
    };

    ParsedContent {
        text: clean_text.trim().to_string(),
        buttons,
        tags,
        random_parts: final_random_parts,
    }
}

/// Parse random content separated by %%%
pub fn parse_random_parts(input: &str) -> Vec<String> {
    input
        .split("%%%")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Parse buttons from text.
///
/// Syntax:
/// - `{button:Text|url}` - Single button
/// - `{button:A|url}:{button:B|url}` - Same row (colon joins)
/// - `{button:A|url} {button:B|url}` - Different rows
///
/// Returns (text without buttons, parsed buttons as rows)
pub fn parse_buttons(input: &str) -> (String, Vec<Vec<InlineButton>>) {
    let mut result_text = String::new();
    let mut rows: Vec<Vec<InlineButton>> = vec![];
    let mut current_row: Vec<InlineButton> = vec![];

    let input_chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < input_chars.len() {
        // Check for {button: pattern
        if input_chars[i] == '{' && i + 7 < input_chars.len() {
            let prefix: String = input_chars[i..i + 8].iter().collect();
            if prefix.to_lowercase() == "{button:" {
                // Try to parse button
                if let Some((btn, end_idx)) = try_parse_button(&input_chars, i) {
                    current_row.push(btn);
                    i = end_idx;
                    
                    // Check what comes after: colon means same row
                    if i < input_chars.len() && input_chars[i] == ':' {
                        i += 1;
                        continue;
                    } else {
                        if !current_row.is_empty() {
                            rows.push(current_row);
                            current_row = vec![];
                        }
                        continue;
                    }
                }
            }
        }
        result_text.push(input_chars[i]);
        i += 1;
    }

    // Push last row
    if !current_row.is_empty() {
        rows.push(current_row);
    }

    (result_text, rows)
}

/// Try to parse a button: {button:Text|url}
fn try_parse_button(chars: &[char], start: usize) -> Option<(InlineButton, usize)> {
    if start + 8 >= chars.len() {
        return None;
    }

    let prefix: String = chars[start..start + 8].iter().collect();
    if prefix.to_lowercase() != "{button:" {
        return None;
    }

    let mut i = start + 8;

    // Find the | separator
    let mut text = String::new();
    while i < chars.len() && chars[i] != '|' && chars[i] != '}' {
        text.push(chars[i]);
        i += 1;
    }

    if i >= chars.len() || chars[i] != '|' {
        return None;
    }
    i += 1; // skip |

    // Find closing }
    let mut url = String::new();
    while i < chars.len() && chars[i] != '}' {
        url.push(chars[i]);
        i += 1;
    }

    if i >= chars.len() || chars[i] != '}' {
        return None;
    }
    i += 1; // skip }

    // Validate
    let text = text.trim().to_string();
    let url = url.trim().to_string();
    if text.is_empty() || url.is_empty() {
        return None;
    }

    Some((InlineButton::new(text, url), i))
}

/// Parse tags from text.
///
/// Tags: `{admin}`, `{user}`, `{private}`, `{noprivate}`, `{protect}`,
///       `{preview}`, `{nonotif}`, `{mediaspoiler}`, `{replytag}`
///
/// Returns (text without tags, parsed tags)
pub fn parse_tags(input: &str) -> (String, ContentTags) {
    let mut tags = ContentTags::default();
    let mut text = input.to_string();

    if text.contains("{admin}") {
        tags.admin_only = true;
        text = text.replace("{admin}", "");
    }
    if text.contains("{user}") {
        tags.user_only = true;
        text = text.replace("{user}", "");
    }
    if text.contains("{private}") {
        tags.is_private = true;
        text = text.replace("{private}", "");
    }
    if text.contains("{noprivate}") {
        tags.no_private = true;
        text = text.replace("{noprivate}", "");
    }
    if text.contains("{protect}") {
        tags.protect = true;
        text = text.replace("{protect}", "");
    }
    if text.contains("{preview}") {
        tags.preview = true;
        text = text.replace("{preview}", "");
    }
    if text.contains("{nonotif}") {
        tags.no_notif = true;
        text = text.replace("{nonotif}", "");
    }
    if text.contains("{mediaspoiler}") {
        tags.media_spoiler = true;
        text = text.replace("{mediaspoiler}", "");
    }
    if text.contains("{replytag}") {
        tags.replytag = true;
        text = text.replace("{replytag}", "");
    }

    (text, tags)
}

/// Apply fillings (placeholders) to text.
///
/// Fillings:
/// - `{first}` / `{firstname}` - First name
/// - `{last}` / `{lastname}` - Last name  
/// - `{fullname}` - Full name
/// - `{username}` - @username or mention
/// - `{mention}` - Mention with name
/// - `{id}` - User ID
/// - `{chatname}` / `{group}` - Chat name
/// - `{count}` - Member count (if provided)
pub fn apply_fillings(text: &str, user: &User, chat_name: &str, count: Option<u64>) -> String {
    let first = &user.first_name;
    let last = user.last_name.as_deref().unwrap_or("");
    let fullname = if last.is_empty() {
        first.clone()
    } else {
        format!("{} {}", first, last)
    };
    let username = user
        .username
        .as_ref()
        .map(|u| format!("@{}", u))
        .unwrap_or_else(|| {
            format!(
                "<a href=\"tg://user?id={}\">{}</a>",
                user.id,
                html_escape(first)
            )
        });
    let mention = format!(
        "<a href=\"tg://user?id={}\">{}</a>",
        user.id,
        html_escape(first)
    );
    let count_str = count.map(|c| c.to_string()).unwrap_or_default();

    text.replace("{first}", &html_escape(first))
        .replace("{firstname}", &html_escape(first))
        .replace("{last}", &html_escape(last))
        .replace("{lastname}", &html_escape(last))
        .replace("{fullname}", &html_escape(&fullname))
        .replace("{username}", &username)
        .replace("{mention}", &mention)
        .replace("{id}", &user.id.to_string())
        .replace("{chatname}", &html_escape(chat_name))
        .replace("{group}", &html_escape(chat_name))
        .replace("{count}", &count_str)
}

/// Apply {rules} filling - creates a deep link button.
/// Returns (text, additional rule buttons with same_row flag)
pub fn _apply_rules_filling(
    text: &str,
    chat_id: i64,
    bot_username: &str,
) -> (String, Vec<(InlineButton, bool)>) {
    let mut result = text.to_string();
    let mut buttons = vec![];

    let rules_url = format!("https://t.me/{}?start=rules_{}", bot_username, chat_id);

    // {rules:same} - button on same row as previous
    if result.contains("{rules:same}") {
        result = result.replace("{rules:same}", "");
        buttons.push((InlineButton::new("📜 Peraturan", &rules_url), true));
    }

    // {rules} - new row button
    if result.contains("{rules}") {
        result = result.replace("{rules}", "");
        buttons.push((InlineButton::new("📜 Peraturan", &rules_url), false));
    }

    (result, buttons)
}

/// Escape HTML special characters.
pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Format relative time duration in Indonesian.
pub fn _format_duration_id(secs: u64) -> String {
    if secs < 60 {
        format!("{} detik", secs)
    } else if secs < 3600 {
        format!("{} menit", secs / 60)
    } else if secs < 86400 {
        format!("{} jam", secs / 3600)
    } else {
        format!("{} hari", secs / 86400)
    }
}

/// Format relative time duration with more detail (hours + minutes).
pub fn format_duration_full(secs: u64) -> String {
    if secs < 60 {
        format!("{} detik", secs)
    } else if secs < 3600 {
        let mins = secs / 60;
        format!("{} menit", mins)
    } else if secs < 86400 {
        let hours = secs / 3600;
        let mins = (secs % 3600) / 60;
        if mins > 0 {
            format!("{} jam {} menit", hours, mins)
        } else {
            format!("{} jam", hours)
        }
    } else {
        let days = secs / 86400;
        let hours = (secs % 86400) / 3600;
        if hours > 0 {
            format!("{} hari {} jam", days, hours)
        } else {
            format!("{} hari", days)
        }
    }
}

/// Parse duration string (e.g., "1h", "30m", "1d").
///
/// Supported units:
/// - m: minutes
/// - h: hours
/// - d: days
/// - w: weeks
pub fn parse_duration(input: &str) -> Option<std::time::Duration> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }

    let (digits, unit) = input.split_at(input.len() - 1);
    let amount: u64 = digits.parse().ok()?;

    let seconds = match unit {
        "m" => amount * 60,
        "h" => amount * 3600,
        "d" => amount * 86400,
        "w" => amount * 604800,
        _ => return None,
    };

    Some(std::time::Duration::from_secs(seconds))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_buttons_colon_syntax() {
        let input = "{button:A|url1}:{button:B|url2}";
        let (_, buttons) = parse_buttons(input);
        
        assert_eq!(buttons.len(), 1); // One row
        assert_eq!(buttons[0].len(), 2); // Two buttons
    }

    #[test]
    fn test_parse_tags() {
        let input = "Hello {admin} {user} world";
        let (text, tags) = parse_tags(input);
        
        assert!(tags.admin_only);
        assert!(tags.user_only);
        assert!(text.contains("Hello"));
    }

    #[test]
    fn test_random_parts() {
        let input = "Part one\n%%%\nPart two\n%%%\nPart three";
        let parts = parse_random_parts(input);
        
        assert_eq!(parts.len(), 3);
    }

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("30m"), Some(std::time::Duration::from_secs(1800)));
        assert_eq!(parse_duration("1h"), Some(std::time::Duration::from_secs(3600)));
        assert_eq!(parse_duration("1d"), Some(std::time::Duration::from_secs(86400)));
        assert_eq!(parse_duration("1w"), Some(std::time::Duration::from_secs(604800)));
        assert_eq!(parse_duration("invalid"), None);
    }
}
