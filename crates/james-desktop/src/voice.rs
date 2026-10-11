//! Hands-free conversation turns: listen, ask, answer aloud, listen again.
//! Voice can ask questions and hear answers. It can never approve or execute
//! desktop input; every action still goes through the review screen.
use std::time::Duration;

/// Silence allowed before a listening turn ends and the conversation pauses.
pub const LISTEN: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Off,
    Listening,
    Thinking,
    Speaking,
}

impl Phase {
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Listening => "Listening…",
            Self::Thinking => "Thinking…",
            Self::Speaking => "Speaking…",
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Heard {
    Nothing,
    Stop,
    Ask(String),
}

const STOP_PHRASES: &[&str] = &[
    "stop",
    "stop listening",
    "stop talking",
    "pause",
    "bye",
    "goodbye",
    "good bye",
    "that's all",
    "that is all",
    "thats all",
    "never mind",
    "nevermind",
    "end conversation",
    "thank you that's all",
    "thanks that's all",
];

/// Classifies one recognized utterance. Only a whole-utterance stop phrase
/// ends the conversation, so "stop the timer in this app" is still a question.
pub fn interpret(transcript: &str) -> Heard {
    let normalized: String = transcript
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '\'' {
                c
            } else {
                ' '
            }
        })
        .collect();
    let words: Vec<&str> = normalized.split_whitespace().collect();
    if words.is_empty() {
        return Heard::Nothing;
    }
    let mut phrase = words.join(" ");
    for prefix in ["hey james ", "okay james ", "ok james ", "james "] {
        if let Some(rest) = phrase.strip_prefix(prefix) {
            phrase = rest.to_owned();
            break;
        }
    }
    if STOP_PHRASES.contains(&phrase.as_str()) {
        Heard::Stop
    } else {
        Heard::Ask(transcript.trim().to_owned())
    }
}

/// Turns a Markdown reply into text that sounds natural when read aloud.
/// Code blocks are summarized rather than spelled out symbol by symbol.
pub fn speakable(markdown: &str) -> String {
    let mut out = String::new();
    let mut in_code = false;
    for line in markdown.lines() {
        let line = line.trim();
        if line.starts_with("```") {
            if !in_code {
                out.push_str(" The code is shown in the conversation. ");
            }
            in_code = !in_code;
            continue;
        }
        if in_code {
            continue;
        }
        let line = line.trim_start_matches(['#', '>']).trim_start();
        let line = ["- ", "* ", "+ "]
            .iter()
            .find_map(|bullet| line.strip_prefix(bullet))
            .unwrap_or(line);
        out.push_str(&links_to_text(line));
        out.push(' ');
    }
    let cleaned: String = out
        .chars()
        .filter(|c| !matches!(c, '*' | '`'))
        .map(|c| if c == '_' { ' ' } else { c })
        .collect();
    let spoken = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    if spoken.is_empty() {
        "My answer is in the conversation.".into()
    } else {
        spoken
    }
}

/// `[label](url)` becomes `label`; nobody wants a URL read aloud.
fn links_to_text(line: &str) -> String {
    let mut out = String::new();
    let mut rest = line;
    while let Some(open) = rest.find('[') {
        let after = &rest[open + 1..];
        if let Some(close) = after.find("](")
            && let Some(end) = after[close + 2..].find(')')
        {
            out.push_str(&rest[..open]);
            out.push_str(&after[..close]);
            rest = &after[close + 2 + end + 1..];
            continue;
        }
        out.push_str(&rest[..=open]);
        rest = after;
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_and_stop_phrases_end_the_conversation() {
        assert_eq!(interpret(""), Heard::Nothing);
        assert_eq!(interpret("  ...  "), Heard::Nothing);
        for phrase in [
            "Stop.",
            "goodbye",
            "James, stop listening",
            "That's all!",
            "Hey James never mind",
        ] {
            assert_eq!(interpret(phrase), Heard::Stop, "{phrase}");
        }
    }

    #[test]
    fn commands_inside_questions_are_still_questions() {
        assert_eq!(
            interpret("How do I stop the timer in this app?"),
            Heard::Ask("How do I stop the timer in this app?".into())
        );
        // Voice has no approval vocabulary: this is only a chat message.
        assert_eq!(interpret("allow once"), Heard::Ask("allow once".into()));
    }

    #[test]
    fn markdown_is_read_naturally() {
        let reply = "## Next steps\n- Open **Settings**\n- See [the docs](https://example.com/x)\n```rust\nfn main() {}\n```\nUse `file_name`.";
        assert_eq!(
            speakable(reply),
            "Next steps Open Settings See the docs The code is shown in the conversation. Use file name."
        );
        assert_eq!(
            speakable("```\nonly code\n```"),
            "The code is shown in the conversation."
        );
        assert_eq!(speakable("   "), "My answer is in the conversation.");
        assert_eq!(links_to_text("a [b] c [d](e"), "a [b] c [d](e");
    }
}
