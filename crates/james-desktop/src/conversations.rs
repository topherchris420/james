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

const MINUTE_MS: u64 = 60_000;
const HOUR_MS: u64 = 60 * MINUTE_MS;
const DAY_MS: u64 = 24 * HOUR_MS;
const WEEK_MS: u64 = 7 * DAY_MS;

/// Elapsed-time groups, independent of local calendar dates or time zones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryGroup {
    Recent,
    PreviousWeek,
    Earlier,
    UnknownTime,
}

impl HistoryGroup {
    /// Display order, newest elapsed window first; unknown times last.
    pub const ORDER: [Self; 4] = [
        Self::Recent,
        Self::PreviousWeek,
        Self::Earlier,
        Self::UnknownTime,
    ];

    pub fn rank(self) -> u8 {
        match self {
            Self::Recent => 0,
            Self::PreviousWeek => 1,
            Self::Earlier => 2,
            Self::UnknownTime => 3,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Recent => "Recent (under 24 hours)",
            Self::PreviousWeek => "Previous week (24 hours to under 7 days)",
            Self::Earlier => "Earlier (7 days or more)",
            Self::UnknownTime => "Unknown time",
        }
    }
}

fn elapsed(updated_at: u64, now: u64) -> Option<u64> {
    if updated_at == 0 {
        None
    } else {
        now.checked_sub(updated_at)
    }
}

/// Classifies a saved Unix-millisecond timestamp without consulting the clock.
/// Zero and future timestamps are unknown, including after a clock adjustment.
pub fn group(updated_at: u64, now: u64) -> HistoryGroup {
    match elapsed(updated_at, now) {
        Some(age) if age < DAY_MS => HistoryGroup::Recent,
        Some(age) if age < WEEK_MS => HistoryGroup::PreviousWeek,
        Some(_) => HistoryGroup::Earlier,
        None => HistoryGroup::UnknownTime,
    }
}

/// A coarse elapsed-time label. Inputs are Unix milliseconds; future or missing
/// timestamps never appear as freshly saved conversations.
pub fn age_label(updated_at: u64, now: u64) -> String {
    match elapsed(updated_at, now) {
        None => "Time unavailable".to_owned(),
        Some(age) if age < MINUTE_MS => "Just saved".to_owned(),
        Some(age) if age < HOUR_MS => format!("{} min ago", age / MINUTE_MS),
        Some(age) if age < DAY_MS => format!("{} hr ago", age / HOUR_MS),
        Some(age) => {
            let days = age / DAY_MS;
            format!("{days} {} ago", if days == 1 { "day" } else { "days" })
        }
    }
}

/// One bounded history row: metadata only, never messages, screenshots or drafts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryRow {
    pub id: String,
    pub title: String,
    pub message_count: usize,
    pub updated_at: u64,
    pub group: HistoryGroup,
}

/// Literal, case-insensitive local search. Every query word must occur in
/// the title or a retained message; no files or model server are consulted.
pub fn matches(conversation: &Conversation, query: &str) -> bool {
    let terms: Vec<_> = query.split_whitespace().map(str::to_lowercase).collect();
    if terms.is_empty() {
        return true;
    }
    let text = std::iter::once(conversation.title.to_lowercase())
        .chain(
            conversation
                .messages
                .iter()
                .map(|m| m.content.to_lowercase()),
        )
        .collect::<Vec<_>>()
        .join("\n");
    terms.iter().all(|term| text.contains(term))
}

/// Prepares the saved-history rows shown under the search box, once per frame.
/// Rows are the archived conversations that literally match `query`, classified
/// against `now` and sorted by group order, then newest timestamp, then id.
/// Only metadata is copied and the archives are never changed. The current
/// conversation is not a saved row, so it stays outside these results.
pub fn history_rows(archives: &[Conversation], query: &str, now: u64) -> Vec<HistoryRow> {
    let mut rows: Vec<HistoryRow> = archives
        .iter()
        .filter(|conversation| matches(conversation, query))
        .map(|conversation| HistoryRow {
            id: conversation.id.clone(),
            title: conversation.title.clone(),
            message_count: conversation.messages.len(),
            updated_at: conversation.updated_at,
            group: group(conversation.updated_at, now),
        })
        .collect();
    rows.sort_by(|left, right| {
        left.group
            .rank()
            .cmp(&right.group.rank())
            .then_with(|| right.updated_at.cmp(&left.updated_at))
            .then_with(|| left.id.cmp(&right.id))
    });
    rows
}

/// Derives the same bounded first-question title used for saved conversations.
/// This only reads the supplied messages and does not archive or persist them.
pub fn current_title(messages: &[ChatMessage]) -> String {
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
        title: current_title(&messages),
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
        conversation.title = current_title(&messages);
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
    fn history_groups_use_elapsed_windows_with_exact_boundaries() {
        let now = 10 * DAY_MS;
        for (age, expected) in [
            (0, HistoryGroup::Recent),
            (DAY_MS - 1, HistoryGroup::Recent),
            (DAY_MS, HistoryGroup::PreviousWeek),
            (WEEK_MS - 1, HistoryGroup::PreviousWeek),
            (WEEK_MS, HistoryGroup::Earlier),
            (WEEK_MS + 1, HistoryGroup::Earlier),
        ] {
            assert_eq!(group(now - age, now), expected, "age: {age}");
        }
        assert_eq!(HistoryGroup::Recent.label(), "Recent (under 24 hours)");
        assert_eq!(
            HistoryGroup::PreviousWeek.label(),
            "Previous week (24 hours to under 7 days)"
        );
        assert_eq!(HistoryGroup::Earlier.label(), "Earlier (7 days or more)");
        assert_eq!(HistoryGroup::UnknownTime.label(), "Unknown time");
    }

    #[test]
    fn history_age_labels_round_down_without_calendar_assumptions() {
        let now = 10 * DAY_MS;
        for (age, expected) in [
            (0, "Just saved"),
            (MINUTE_MS - 1, "Just saved"),
            (MINUTE_MS, "1 min ago"),
            (5 * MINUTE_MS, "5 min ago"),
            (HOUR_MS - 1, "59 min ago"),
            (HOUR_MS, "1 hr ago"),
            (2 * HOUR_MS, "2 hr ago"),
            (DAY_MS - 1, "23 hr ago"),
            (DAY_MS, "1 day ago"),
            (3 * DAY_MS, "3 days ago"),
            (WEEK_MS, "7 days ago"),
        ] {
            assert_eq!(age_label(now - age, now), expected, "age: {age}");
        }
    }

    #[test]
    fn missing_future_and_extreme_history_times_are_safe() {
        for (timestamp, now) in [
            (0, 0),
            (0, 1),
            (0, u64::MAX),
            (1, 0),
            (2, 1),
            (u64::MAX, 1),
            (u64::MAX, u64::MAX - 1),
        ] {
            assert_eq!(group(timestamp, now), HistoryGroup::UnknownTime);
            assert_eq!(age_label(timestamp, now), "Time unavailable");
        }
        assert_eq!(group(u64::MAX, u64::MAX), HistoryGroup::Recent);
        assert_eq!(age_label(u64::MAX, u64::MAX), "Just saved");
        assert_eq!(group(1, u64::MAX), HistoryGroup::Earlier);
        assert_eq!(
            age_label(1, u64::MAX),
            format!("{} days ago", (u64::MAX - 1) / DAY_MS)
        );
    }

    #[test]
    fn current_title_is_bounded_and_uses_only_the_first_user_question() {
        let messages = vec![
            ChatMessage::new("assistant", "An answer is not a title"),
            ChatMessage::new("user", "  First\nquestion\twith\u{0000} context  "),
            ChatMessage::new("user", "Later question"),
        ];
        let original = messages.clone();
        assert_eq!(current_title(&messages), "First question with context");
        assert_eq!(messages, original);
        assert_eq!(current_title(&[]), "Conversation");
        assert_eq!(
            current_title(&[ChatMessage::new("assistant", "No question")]),
            "Conversation"
        );
        assert_eq!(
            current_title(&[ChatMessage::new("user", "🧪".repeat(100))])
                .chars()
                .count(),
            TITLE_CHARS
        );
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
    fn search_finds_answer_text_and_treats_patterns_literally() {
        let mut saved = chat(1, 1);
        saved
            .messages
            .push(ChatMessage::new("assistant", "Résultat: 🧪 voltage [3.2]"));
        assert!(matches(&saved, "RÉSULTAT voltage"));
        assert!(matches(&saved, "question 🧪"));
        assert!(matches(&saved, "[3.2]"));
        assert!(matches(&saved, "  "));
        assert!(!matches(&saved, "voltage absent"));
        assert!(!matches(&saved, ".*"));
    }

    fn titled(id: &str, updated_at: u64, title: &str) -> Conversation {
        Conversation {
            id: id.into(),
            title: title.into(),
            updated_at,
            messages: vec![ChatMessage::new("user", title)],
        }
    }

    fn ids(rows: &[HistoryRow]) -> Vec<&str> {
        rows.iter().map(|row| row.id.as_str()).collect()
    }

    #[test]
    fn history_rows_sort_groups_then_newest_then_id() {
        let now = 10 * DAY_MS;
        let archives = vec![
            titled("chat-d", now - 9 * DAY_MS, "Older"),
            titled("chat-b", now - 2 * HOUR_MS, "Recent older"),
            titled("chat-f", now - 3 * DAY_MS, "This week"),
            titled("chat-a", now - MINUTE_MS, "Recent newest"),
            titled("chat-c", now - WEEK_MS, "Earlier"),
        ];
        let rows = history_rows(&archives, "", now);
        assert_eq!(
            ids(&rows),
            ["chat-a", "chat-b", "chat-f", "chat-c", "chat-d"]
        );
        assert_eq!(rows[0].group, HistoryGroup::Recent);
        assert_eq!(rows[2].group, HistoryGroup::PreviousWeek);
        assert_eq!(rows[3].group, HistoryGroup::Earlier);
        assert_eq!(rows[0].title, "Recent newest");
        assert_eq!(rows[0].message_count, 1);
        assert_eq!(rows[0].updated_at, now - MINUTE_MS);
        assert_eq!(
            HistoryGroup::ORDER,
            [
                HistoryGroup::Recent,
                HistoryGroup::PreviousWeek,
                HistoryGroup::Earlier,
                HistoryGroup::UnknownTime,
            ]
        );
    }

    #[test]
    fn history_rows_break_tied_timestamps_by_id() {
        let now = DAY_MS;
        let archives = vec![
            titled("chat-m", now / 2, "Tied middle"),
            titled("chat-z", now / 2, "Tied last"),
            titled("chat-j", now / 2, "Tied first"),
        ];
        assert_eq!(
            ids(&history_rows(&archives, "", now)),
            ["chat-j", "chat-m", "chat-z"]
        );
    }

    #[test]
    fn zero_and_future_time_rows_are_unknown_and_grouped_last() {
        let now = 10 * DAY_MS;
        let archives = vec![
            titled("chat-0", 0, "No time"),
            titled("chat-f", now + WEEK_MS, "Future"),
            titled("chat-e", now - WEEK_MS, "Oldest known"),
            titled("chat-r", now - MINUTE_MS, "Just now"),
        ];
        let rows = history_rows(&archives, "", now);
        assert_eq!(
            ids(&rows),
            ["chat-r", "chat-e", "chat-f", "chat-0"],
            "unknown-time rows tie on nothing known, so id decides"
        );
        assert_eq!(rows[2].group, HistoryGroup::UnknownTime);
        assert_eq!(rows[3].group, HistoryGroup::UnknownTime);
    }

    #[test]
    fn history_rows_search_answer_text_literally_and_keep_group_order() {
        let now = WEEK_MS;
        let mut answer = titled("chat-a", now - 2 * HOUR_MS, "Question one");
        answer
            .messages
            .push(ChatMessage::new("assistant", "Résultat: 🧪 voltage [3.2]"));
        let mut absent = titled("chat-b", now - 3 * DAY_MS, "Question two");
        absent
            .messages
            .push(ChatMessage::new("assistant", "A different result"));
        let mut pattern = titled("chat-c", now - MINUTE_MS, "Question three");
        pattern
            .messages
            .push(ChatMessage::new("assistant", "A literal .* pattern"));
        let archives = vec![absent, pattern, answer];
        let rows = history_rows(&archives, " RÉSULTAT [3.2] ", now);
        assert_eq!(ids(&rows), ["chat-a"]);
        assert_eq!(rows[0].message_count, 2);
        assert_eq!(ids(&history_rows(&archives, ".*", now)), ["chat-c"]);
        assert!(
            history_rows(&archives, "result warning", now).is_empty(),
            "every query word must match"
        );
        // One recent chat newer than another recent chat, then a previous-week
        // chat last: group order first, then newest inside each group.
        let all = history_rows(&archives, "question", now);
        assert_eq!(ids(&all), ["chat-c", "chat-a", "chat-b"]);
    }

    #[test]
    fn history_rows_do_not_change_the_archives_or_their_order() {
        let now = 10 * DAY_MS;
        let mut archives = vec![
            titled("chat-2", now - 9 * DAY_MS, "Old"),
            titled("chat-1", now - MINUTE_MS, "New"),
            titled("chat-3", 0, "Unknown"),
        ];
        archives[0]
            .messages
            .push(ChatMessage::new("assistant", "Hidden answer"));
        let before = archives.clone();
        let _ = history_rows(&archives, "hidden", now);
        let _ = history_rows(&archives, "", now);
        let _ = history_rows(&archives, "nothing matches", now);
        assert_eq!(archives, before);
        assert_eq!(
            ids(&history_rows(&archives, "", now)),
            ["chat-1", "chat-2", "chat-3"]
        );
        assert_eq!(archives, before);
        assert!(history_rows(&[], "", now).is_empty());
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
