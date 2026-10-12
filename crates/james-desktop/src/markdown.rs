//! A small Markdown reader for model replies: headings, paragraphs, emphasis,
//! inline code, fenced code blocks, lists, quotes, rules and tables (shown as
//! preformatted text). Anything it does not recognise stays as plain text.
//!
//! Links are model output, so their anchor text is never trusted on its own:
//! the full address is always shown, and nothing opens until the person
//! clicks it.
use eframe::egui::{self, Color32, FontId, RichText, TextFormat, text::LayoutJob};

/// The reasoning a model wrote inside a leading `<think>` block, separated
/// from its answer. Reasoning is shown folded away and never spoken or parsed
/// as an action.
pub fn split_reasoning(text: &str) -> (Option<&str>, &str) {
    let trimmed = text.trim_start();
    let Some(rest) = trimmed.strip_prefix("<think>") else {
        return (None, text);
    };
    match rest.find("</think>") {
        Some(end) => (
            Some(rest[..end].trim()),
            rest[end + "</think>".len()..].trim_start(),
        ),
        // Still reasoning (a reply in progress): everything so far is reasoning.
        None => (Some(rest.trim()), ""),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Span {
    Text(String),
    Strong(String),
    Emphasis(String),
    Code(String),
    Link { label: String, url: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Heading(u8, Vec<Span>),
    Paragraph(Vec<Span>),
    /// One list item: nesting depth, marker ("•" or "3."), content.
    Item(usize, String, Vec<Span>),
    Quote(Vec<Span>),
    Code {
        language: String,
        text: String,
    },
    /// Lines kept exactly as written, such as a table.
    Preformatted(String),
    Rule,
}

/// Splits Markdown into blocks. Unclosed code fences (a reply still
/// streaming) run to the end of the text.
pub fn parse(text: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut paragraph: Vec<&str> = Vec::new();
    let mut lines = text.lines().peekable();
    let flush = |paragraph: &mut Vec<&str>, blocks: &mut Vec<Block>| {
        if !paragraph.is_empty() {
            blocks.push(Block::Paragraph(inline(&paragraph.join(" "))));
            paragraph.clear();
        }
    };
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if let Some(fence) = trimmed
            .strip_prefix("```")
            .or_else(|| trimmed.strip_prefix("~~~"))
        {
            flush(&mut paragraph, &mut blocks);
            let marker = &trimmed[..3];
            let mut code = Vec::new();
            for next in lines.by_ref() {
                if next.trim_start().starts_with(marker) {
                    break;
                }
                code.push(next);
            }
            blocks.push(Block::Code {
                language: fence.trim().chars().take(24).collect(),
                text: code.join("\n"),
            });
            continue;
        }
        if trimmed.is_empty() {
            flush(&mut paragraph, &mut blocks);
            continue;
        }
        if trimmed.starts_with('|') {
            flush(&mut paragraph, &mut blocks);
            let mut table = vec![trimmed];
            while let Some(next) = lines.peek() {
                if !next.trim().starts_with('|') {
                    break;
                }
                table.push(next.trim());
                lines.next();
            }
            blocks.push(Block::Preformatted(table.join("\n")));
            continue;
        }
        let compact: Vec<char> = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
        if compact.len() >= 3
            && ['-', '*', '_']
                .iter()
                .any(|rule| compact.iter().all(|c| c == rule))
        {
            flush(&mut paragraph, &mut blocks);
            blocks.push(Block::Rule);
            continue;
        }
        let hashes = trimmed.chars().take_while(|c| *c == '#').count();
        if (1..=6).contains(&hashes) && trimmed[hashes..].starts_with(' ') {
            flush(&mut paragraph, &mut blocks);
            blocks.push(Block::Heading(
                hashes as u8,
                inline(trimmed[hashes..].trim()),
            ));
            continue;
        }
        if let Some(quote) = trimmed.strip_prefix('>') {
            flush(&mut paragraph, &mut blocks);
            blocks.push(Block::Quote(inline(quote.trim())));
            continue;
        }
        let depth = (line.len() - line.trim_start().len()) / 2;
        if let Some(item) = ["- ", "* ", "+ "]
            .iter()
            .find_map(|b| trimmed.strip_prefix(b))
        {
            flush(&mut paragraph, &mut blocks);
            blocks.push(Block::Item(depth, "•".into(), inline(item)));
            continue;
        }
        let digits = trimmed.chars().take_while(char::is_ascii_digit).count();
        if (1..=3).contains(&digits)
            && let Some(item) = trimmed[digits..]
                .strip_prefix(". ")
                .or_else(|| trimmed[digits..].strip_prefix(") "))
        {
            flush(&mut paragraph, &mut blocks);
            blocks.push(Block::Item(
                depth,
                format!("{}.", &trimmed[..digits]),
                inline(item),
            ));
            continue;
        }
        paragraph.push(trimmed);
    }
    flush(&mut paragraph, &mut blocks);
    blocks
}

/// Inline emphasis, code and links. Unmatched markers stay as literal text.
pub fn inline(text: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut plain = String::new();
    let mut rest = text;
    let push_plain = |plain: &mut String, spans: &mut Vec<Span>| {
        if !plain.is_empty() {
            spans.push(Span::Text(std::mem::take(plain)));
        }
    };
    while let Some(c) = rest.chars().next() {
        let (marker, kind): (&str, fn(String) -> Span) = if rest.starts_with("**") {
            ("**", Span::Strong)
        } else if rest.starts_with("__") {
            ("__", Span::Strong)
        } else if c == '`' {
            ("`", Span::Code)
        } else if c == '*' {
            ("*", Span::Emphasis)
        } else if c == '[' {
            if let Some((label, url, used)) = link(rest) {
                push_plain(&mut plain, &mut spans);
                spans.push(Span::Link { label, url });
                rest = &rest[used..];
                continue;
            }
            plain.push(c);
            rest = &rest[1..];
            continue;
        } else {
            plain.push(c);
            rest = &rest[c.len_utf8()..];
            continue;
        };
        let body = &rest[marker.len()..];
        match body.find(marker) {
            Some(end) if end > 0 && !body.starts_with(' ') => {
                push_plain(&mut plain, &mut spans);
                spans.push(kind(body[..end].to_owned()));
                rest = &body[end + marker.len()..];
            }
            _ => {
                plain.push_str(marker);
                rest = body;
            }
        }
    }
    push_plain(&mut plain, &mut spans);
    spans
}

/// `[label](url)` at the start of `text`: label, url and bytes consumed.
fn link(text: &str) -> Option<(String, String, usize)> {
    let close = text.find("](")?;
    let label = &text[1..close];
    if label.contains('[') {
        return None;
    }
    let after = &text[close + 2..];
    let end = after.find(')')?;
    let url = after[..end].trim();
    if url.is_empty() || url.contains(char::is_whitespace) {
        return None;
    }
    Some((label.to_owned(), url.to_owned(), close + 2 + end + 1))
}

/// Only web addresses can be opened; anything else is shown as text.
pub fn openable(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    (lower.starts_with("https://") || lower.starts_with("http://")) && url.len() <= 2048
}

/// Colours the renderer needs from the active theme.
#[derive(Clone, Copy)]
pub struct Style {
    pub text: Color32,
    pub strong: Color32,
    pub accent: Color32,
    pub code_bg: Color32,
    pub weak: Color32,
}

impl Style {
    pub fn from(ui: &egui::Ui) -> Self {
        let visuals = ui.visuals();
        Self {
            text: visuals.text_color(),
            strong: visuals.strong_text_color(),
            accent: visuals.hyperlink_color,
            code_bg: visuals.code_bg_color,
            weak: visuals.weak_text_color(),
        }
    }
}

fn layout(spans: &[Span], style: Style, size: f32, base: Color32) -> LayoutJob {
    let mut job = LayoutJob::default();
    let body = FontId::proportional(size);
    for span in spans {
        let (text, format) = match span {
            Span::Text(t) => (t.as_str(), TextFormat::simple(body.clone(), base)),
            Span::Strong(t) => (t.as_str(), TextFormat::simple(body.clone(), style.strong)),
            Span::Emphasis(t) => (
                t.as_str(),
                TextFormat {
                    italics: true,
                    ..TextFormat::simple(body.clone(), base)
                },
            ),
            Span::Code(t) => (
                t.as_str(),
                TextFormat {
                    background: style.code_bg,
                    ..TextFormat::simple(FontId::monospace(size * 0.92), base)
                },
            ),
            Span::Link { label, .. } => (
                label.as_str(),
                TextFormat {
                    underline: egui::Stroke::new(1.0_f32, style.accent),
                    ..TextFormat::simple(body.clone(), style.accent)
                },
            ),
        };
        job.append(text, 0.0, format);
    }
    job
}

/// Lists a paragraph's links under it with their full addresses.
fn links_ui(ui: &mut egui::Ui, spans: &[Span]) {
    for span in spans {
        if let Span::Link { label, url } = span {
            ui.horizontal_wrapped(|ui| {
                ui.add_space(12.0);
                if openable(url) {
                    ui.hyperlink_to(RichText::new(url).small(), url)
                        .on_hover_text(format!(
                            "“{label}” links to {url}. It opens in your web browser."
                        ));
                } else {
                    ui.small(format!("{label}: {url}"));
                }
            });
        }
    }
}

/// Draws Markdown. `id` keeps folded sections stable between frames.
pub fn show(ui: &mut egui::Ui, text: &str, id: impl std::hash::Hash) {
    let style = Style::from(ui);
    let size = ui
        .style()
        .text_styles
        .get(&egui::TextStyle::Body)
        .map_or(14.0, |font| font.size);
    let (reasoning, answer) = split_reasoning(text);
    let id = ui.make_persistent_id(id);
    if let Some(reasoning) = reasoning.filter(|r| !r.is_empty()) {
        egui::CollapsingHeader::new(RichText::new("Model's reasoning (not checked)").small())
            .id_salt(id.with("reasoning"))
            .default_open(false)
            .show(ui, |ui| {
                ui.label(RichText::new(reasoning).small().color(style.weak));
            });
    }
    for (index, block) in parse(answer).iter().enumerate() {
        match block {
            Block::Heading(level, spans) => {
                ui.add_space(4.0);
                let scale = match level {
                    1 => 1.35,
                    2 => 1.2,
                    _ => 1.08,
                };
                ui.label(layout(spans, style, size * scale, style.strong));
            }
            Block::Paragraph(spans) => {
                ui.label(layout(spans, style, size, style.text));
                links_ui(ui, spans);
            }
            Block::Item(depth, marker, spans) => {
                // Wrapped lines hang under the item's text, not its number.
                ui.horizontal_top(|ui| {
                    ui.add_space(8.0 + 16.0 * (*depth).min(4) as f32);
                    ui.label(RichText::new(marker).size(size).color(style.accent));
                    ui.add(egui::Label::new(layout(spans, style, size, style.text)).wrap());
                });
                links_ui(ui, spans);
            }
            Block::Quote(spans) => {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("|").color(style.weak));
                    ui.label(layout(spans, style, size, style.weak));
                });
            }
            Block::Code { language, text } => {
                egui::Frame::new()
                    .fill(style.code_bg)
                    .inner_margin(8.0)
                    .corner_radius(4.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if !language.is_empty() {
                                ui.small(language);
                            }
                            if ui
                                .small_button("Copy code")
                                .on_hover_text("Copy this code block")
                                .clicked()
                            {
                                ui.ctx().copy_text(text.clone());
                            }
                        });
                        ui.push_id(id.with(index), |ui| {
                            egui::ScrollArea::horizontal().show(ui, |ui| {
                                ui.label(RichText::new(text).monospace());
                            });
                        });
                    });
            }
            Block::Preformatted(text) => {
                ui.push_id(id.with(index), |ui| {
                    egui::ScrollArea::horizontal().show(ui, |ui| {
                        ui.label(RichText::new(text).monospace());
                    });
                });
            }
            Block::Rule => {
                ui.separator();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(s: &str) -> Span {
        Span::Text(s.into())
    }

    #[test]
    fn emphasis_code_and_links_are_recognised() {
        assert_eq!(
            inline("Open **Settings**, then *Voice*: `whisper-cli` [docs](https://x.org/a)."),
            vec![
                text("Open "),
                Span::Strong("Settings".into()),
                text(", then "),
                Span::Emphasis("Voice".into()),
                text(": "),
                Span::Code("whisper-cli".into()),
                text(" "),
                Span::Link {
                    label: "docs".into(),
                    url: "https://x.org/a".into()
                },
                text("."),
            ]
        );
    }

    #[test]
    fn unmatched_markers_stay_literal() {
        assert_eq!(inline("2 * 3 = 6"), vec![text("2 * 3 = 6")]);
        assert_eq!(inline("a ** b"), vec![text("a ** b")]);
        assert_eq!(inline("tick ` alone"), vec![text("tick ` alone")]);
        assert_eq!(inline("[not a link] (x)"), vec![text("[not a link] (x)")]);
        assert_eq!(inline("[a](has space)"), vec![text("[a](has space)")]);
        assert_eq!(inline("café ünïcode"), vec![text("café ünïcode")]);
    }

    #[test]
    fn blocks_are_split_the_way_a_reader_expects() {
        let blocks = parse(
            "# Plan\nFirst line\nwraps here.\n\n- one\n  - nested\n2. two\n> quoted\n\n---\n```rust\nfn main() {}\n```\n| a | b |\n|---|---|\n| 1 | 2 |",
        );
        assert_eq!(
            blocks,
            vec![
                Block::Heading(1, vec![text("Plan")]),
                Block::Paragraph(vec![text("First line wraps here.")]),
                Block::Item(0, "•".into(), vec![text("one")]),
                Block::Item(1, "•".into(), vec![text("nested")]),
                Block::Item(0, "2.".into(), vec![text("two")]),
                Block::Quote(vec![text("quoted")]),
                Block::Rule,
                Block::Code {
                    language: "rust".into(),
                    text: "fn main() {}".into()
                },
                Block::Preformatted("| a | b |\n|---|---|\n| 1 | 2 |".into()),
            ]
        );
    }

    #[test]
    fn a_streaming_code_block_runs_to_the_end() {
        assert_eq!(
            parse("Here:\n```\nlet x = 1;"),
            vec![
                Block::Paragraph(vec![text("Here:")]),
                Block::Code {
                    language: String::new(),
                    text: "let x = 1;".into()
                }
            ]
        );
        // A number that merely starts a sentence is not a list.
        assert_eq!(
            parse("2026 was busy."),
            vec![Block::Paragraph(vec![text("2026 was busy.")])]
        );
        assert_eq!(
            parse("#hashtag"),
            vec![Block::Paragraph(vec![text("#hashtag")])]
        );
    }

    #[test]
    fn reasoning_is_separated_from_the_answer() {
        assert_eq!(
            split_reasoning("<think>Check the units.</think>\n\nThe answer is 4."),
            (Some("Check the units."), "The answer is 4.")
        );
        assert_eq!(
            split_reasoning("<think>still going"),
            (Some("still going"), "")
        );
        assert_eq!(split_reasoning("No reasoning."), (None, "No reasoning."));
        assert_eq!(
            split_reasoning("The tag <think> mid-text is literal."),
            (None, "The tag <think> mid-text is literal.")
        );
    }

    #[test]
    fn only_web_links_can_be_opened() {
        assert!(openable("https://lmstudio.ai/"));
        assert!(openable("HTTP://example.com"));
        for url in [
            "javascript:alert(1)",
            "file:///etc/passwd",
            "ms-settings:privacy",
            "data:text/html,x",
        ] {
            assert!(!openable(url), "{url}");
        }
    }
}
