//! Note parser utilities.
//!
//! Handles parsing of note content including:
//! - Buttons: `{button:Text|url}` and `{button:Text|url}:same`
//! - Tags: `{admin}`, `{private}`, `{protect}`, etc.
//! - Fillings: `{first}`, `{mention}`, `{chatname}`, etc.

use teloxide::types::User;

use crate::database::{InlineButton, NoteTags};

/// Result of parsing note content.
#[derive(Debug, Clone)]
pub struct ParsedNote {
    /// Clean text without buttons and tags
    pub text: String,
    /// Extracted buttons (rows)
    pub buttons: Vec<Vec<InlineButton>>,
    /// Extracted tags
    pub tags: NoteTags,
}

/// Parse note content to extract buttons, tags, and clean text.
pub fn parse_note_content(input: &str) -> ParsedNote {
    let (text_without_buttons, buttons) = parse_buttons(input);
    let (clean_text, tags) = parse_tags(&text_without_buttons);

    ParsedNote {
        text: clean_text.trim().to_string(),
        buttons,
        tags,
    }
}

/// Parse buttons from text.
///
/// Syntax:
/// - `{button:Text|url}` - Single button
/// - `{button:A|url}:{button:B|url}` - Multiple buttons on SAME row (colon joins)
/// - `{button:A|url} {button:B|url}` - Different rows (space/newline separates)
///
/// Example:
/// - `{button:A|url}:{button:B|url}` → [A] [B] (same row)
/// - `{button:A|url} {button:B|url}` → [A] then [B] (different rows)
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
                if let Some((btn, end_idx)) = try_parse_button_simple(&input_chars, i) {
                    current_row.push(btn);
                    i = end_idx;
                    
                    // Check what comes after: colon means same row, else new row
                    if i < input_chars.len() && input_chars[i] == ':' {
                        // Skip colon, continue to next button (same row)
                        i += 1;
                        continue;
                    } else {
                        // Space, newline, or other - push row and start new one
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
/// Returns (InlineButton, end_index)
fn try_parse_button_simple(chars: &[char], start: usize) -> Option<(InlineButton, usize)> {
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
/// Tags: `{admin}`, `{private}`, `{noprivate}`, `{protect}`,
///       `{preview}`, `{nonotif}`, `{mediaspoiler}`
///
/// Returns (text without tags, parsed tags)
pub fn parse_tags(input: &str) -> (String, NoteTags) {
    let mut tags = NoteTags::default();
    let mut text = input.to_string();

    // Check and remove each tag
    if text.contains("{admin}") {
        tags.admin_only = true;
        text = text.replace("{admin}", "");
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

    (text, tags)
}

/// Apply fillings (placeholders) to note text.
///
/// Fillings:
/// - `{first}` - First name
/// - `{last}` - Last name
/// - `{fullname}` - Full name
/// - `{username}` - @username or mention
/// - `{mention}` - Mention with name
/// - `{id}` - User ID
/// - `{chatname}` - Chat name
pub fn apply_fillings(text: &str, user: &User, chat_name: &str, _bot_username: &str) -> String {
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
        .unwrap_or_else(|| format!("<a href=\"tg://user?id={}\">{}</a>", user.id, html_escape(first)));
    let mention = format!(
        "<a href=\"tg://user?id={}\">{}</a>",
        user.id,
        html_escape(first)
    );

    text.replace("{first}", &html_escape(first))
        .replace("{last}", &html_escape(last))
        .replace("{fullname}", &html_escape(&fullname))
        .replace("{username}", &username)
        .replace("{mention}", &mention)
        .replace("{id}", &user.id.to_string())
        .replace("{chatname}", &html_escape(chat_name))
}

/// Apply {rules} and {rules:same} fillings.
/// Returns (text, additional rule buttons to add)
pub fn apply_rules_filling(
    text: &str,
    chat_id: i64,
    bot_username: &str,
) -> (String, Vec<(InlineButton, bool)>) {
    let mut result = text.to_string();
    let mut buttons = vec![];

    let rules_url = format!("https://t.me/{}?start=rules_{}", bot_username, chat_id);

    if result.contains("{rules:same}") {
        result = result.replace("{rules:same}", "");
        buttons.push((InlineButton::new("📜 Peraturan", &rules_url), true));
    }

    if result.contains("{rules}") {
        result = result.replace("{rules}", "");
        buttons.push((InlineButton::new("📜 Peraturan", &rules_url), false));
    }

    (result, buttons)
}

/// Escape HTML special characters.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_buttons_new_syntax() {
        let input = "Hello {button:Click|https://example.com} world {button:More|url}:same";
        let (text, buttons) = parse_buttons(input);

        assert_eq!(text.trim(), "Hello  world");
        assert_eq!(buttons.len(), 1); // Both on same row due to :same
        assert_eq!(buttons[0].len(), 2);
        assert_eq!(buttons[0][0].text, "Click");
        assert_eq!(buttons[0][1].text, "More");
    }

    #[test]
    fn test_parse_buttons_multiple_rows() {
        let input = "{button:Row1|url1}\n{button:Row2|url2}";
        let (_, buttons) = parse_buttons(input);

        assert_eq!(buttons.len(), 2);
        assert_eq!(buttons[0][0].text, "Row1");
        assert_eq!(buttons[1][0].text, "Row2");
    }

    #[test]
    fn test_parse_tags() {
        let input = "Hello {admin} world {protect}";
        let (text, tags) = parse_tags(input);

        assert_eq!(text.trim(), "Hello  world");
        assert!(tags.admin_only);
        assert!(tags.protect);
        assert!(!tags.is_private);
    }
}
