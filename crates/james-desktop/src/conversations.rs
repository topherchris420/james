//! A bounded local library of conversations the person can resume or delete.
//! Screenshots, drafts, model instructions and action records never enter it.
use crate::session::{self, MAX_HISTORY_BYTES, MAX_SAVED_MESSAGES};
use james_models::ChatMessage;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub const MAX_CONVERSATIONS: usize = 20;
const TITLE_CHARS: usize = 80;
const MAX_ID_CHARS: usize = 96;
static NEXT_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    /// Last archived at this Unix time, in milliseconds.
    pub updated_at: u64,
    pub messages: Vec<ChatMessage>,
}

fn title(messages: &[ChatMessage]) -> String {
    let question = messages
        .iter()
        .find(|message| message.role == "user")
        .map(|message| message.content.as_str())
        .unwrap_or_default();
    let title: String = question
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .filter(|c| !c.is_control())
        .take(TITLE_CHARS)
        .collect();
    if title.is_empty() {
        "Conversation".to_owned()
    } else {
        title
    }
}

/// Saves a nonempty conversation and returns its new local id. Callers choose
/// whether history is enabled; the file writer separately enforces that choice.
pub fn archive(archives: &mut Vec<Conversation>, messages: &[ChatMessage]) -> Option<String> {
    let messages = session::bounded(messages.to_vec());
    if messages.is_empty() {
        return None;
    }
    let updated_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX);
    let id = loop {
        let id = format!(
            "chat-{updated_at}-{}",
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        );
        if !archives.iter().any(|conversation| conversation.id == id) {
            break id;
        }
    };
    archives.push(Conversation {
        id: id.clone(),
        title: title(&messages),
        updated_at,
        messages,
    });
    let (_, kept) = bounded(Vec::new(), std::mem::take(archives));
    *archives = kept;
    Some(id)
}

/// Takes an archived conversation out of the library to make it current.
/// A missing id leaves the library untouched.
pub fn restore(archives: &mut Vec<Conversation>, id: &str) -> Option<Vec<ChatMessage>> {
    let index = archives
        .iter()
        .position(|conversation| conversation.id == id)?;
    Some(archives.remove(index).messages)
}

pub fn remove(archives: &mut Vec<Conversation>, id: &str) -> bool {
    let before = archives.len();
    archives.retain(|conversation| conversation.id != id);
    archives.len() != before
}

/// Retention applies to the entire library, including the current conversation:
/// at most 80 messages and 512 KiB of text. Current messages have priority, then
/// the most recently archived chats. Metadata is bounded separately so escaped
/// JSON stays within the session writer's 4 MiB cap.
pub fn bounded(
    current: Vec<ChatMessage>,
    mut archives: Vec<Conversation>,
) -> (Vec<ChatMessage>, Vec<Conversation>) {
    let current = session::bounded(current);
    let mut remaining_messages = MAX_SAVED_MESSAGES.saturating_sub(current.len());
    let mut remaining_bytes =
        MAX_HISTORY_BYTES.saturating_sub(current.iter().map(|message| message.content.len()).sum());
    archives.sort_by(|left, right| {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then_with(|| left.id.cmp(&right.id))
    });
    let mut seen = HashSet::new();
    let mut kept = Vec::new();
    for mut conversation in archives {
        if kept.len() >= MAX_CONVERSATIONS || remaining_messages == 0 {
            break;
        }
        if conversation.id.is_empty()
            || conversation.id.len() > MAX_ID_CHARS
            || !conversation
                .id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'))
            || !seen.insert(conversation.id.clone())
        {
            continue;
        }
        let mut messages: Vec<_> = session::bounded(conversation.messages)
            .into_iter()
            .rev()
            .take(remaining_messages)
            .take_while(|message| {
                let bytes = message.content.len();
                if bytes > remaining_bytes {
                    return false;
                }
                remaining_bytes -= bytes;
                true
            })
            .collect();
        if messages.is_empty() {
            continue;
        }
        messages.reverse();
        remaining_messages -= messages.len();
        conversation.title = title(&messages);
        conversation.messages = messages;
        kept.push(conversation);
    }
    (current, kept)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chat(id: usize, updated_at: u64) -> Conversation {
        Conversation {
            id: format!("chat-{id}"),
            title: "untrusted saved title".into(),
            updated_at,
            messages: vec![ChatMessage::new("user", format!("Question {id}"))],
        }
    }

    #[test]
    fn archive_restore_and_delete_keep_other_conversations() {
        let mut archives = Vec::new();
        assert!(archive(&mut archives, &[]).is_none());
        let messages = vec![
            ChatMessage::new("user", "  First question\nwith context  "),
            ChatMessage::new("assistant", "An answer"),
        ];
        let first = archive(&mut archives, &messages).unwrap();
        let second = archive(&mut archives, &[ChatMessage::new("user", "Another chat")]).unwrap();
        assert_ne!(first, second);
        assert_eq!(archives.len(), 2);
        assert_eq!(
            archives.iter().find(|chat| chat.id == first).unwrap().title,
            "First question with context"
        );
        assert!(restore(&mut archives, "missing").is_none());
        assert_eq!(restore(&mut archives, &first), Some(messages));
        assert_eq!(archives.len(), 1);
        assert!(remove(&mut archives, &second));
        assert!(!remove(&mut archives, &second));
        assert!(archives.is_empty());
    }

    #[test]
    fn newest_chats_are_bounded_together_with_the_current_conversation() {
        let current = vec![ChatMessage::new("user", "Current")];
        let archives: Vec<_> = (0..50).map(|id| chat(id, id as u64)).collect();
        let (kept_current, kept) = bounded(current.clone(), archives);
        assert_eq!(kept_current, current);
        assert_eq!(kept.len(), MAX_CONVERSATIONS);
        assert_eq!(kept[0].id, "chat-49");
        assert_eq!(kept.last().unwrap().id, "chat-30");
        let mut archived = chat(100, 100);
        archived.messages = (0..80)
            .map(|_| ChatMessage::new("assistant", "answer"))
            .collect();
        let (current, kept) = bounded(kept_current, vec![archived]);
        assert_eq!(current.len() + kept[0].messages.len(), MAX_SAVED_MESSAGES);
        assert_eq!(bounded(current.clone(), kept.clone()), (current, kept));
    }

    #[test]
    fn malformed_ids_duplicate_chats_and_unbounded_titles_are_not_restored() {
        let mut long = chat(1, 1);
        long.messages = vec![ChatMessage::new("user", "🧪".repeat(100))];
        let mut invalid = chat(2, 2);
        invalid.id = "not/a/local/id".into();
        let mut duplicate = long.clone();
        duplicate.updated_at = 2;
        let (_, kept) = bounded(Vec::new(), vec![long, duplicate, invalid]);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].title.chars().count(), TITLE_CHARS);
        assert_eq!(kept[0].updated_at, 2);
    }
}
