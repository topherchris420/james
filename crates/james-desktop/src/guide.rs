//! Guide me markers: where on the screenshot each suggested step's control
//! is. The model adds one machine-readable line to its answer; James removes
//! it before the answer is shown, saved or spoken, and draws numbered marks
//! on its own copy of the screenshot. Marks only point: they never act, and
//! nothing is drawn over the live screen.

/// Asks for the Marks line, after the attached screenshot's dimensions.
pub const MARKS_CONTEXT: &str = "After the steps, add one last line in exactly this form, giving where each step's button, field or link is in the attached screenshot, in pixels: Marks: 1 (x, y); 2 (x, y). Leave out a step that has nothing on screen to point at, and write Marks: none if no step has one. James removes this line before showing or reading the answer.";

/// The most steps Guide me suggests, and so the most marks.
const MAX_STEPS: u32 = 3;

/// One numbered mark on the screenshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mark {
    pub step: u32,
    pub x: u32,
    pub y: u32,
}

/// The marks of the latest Guide me answer, in full-screen pixels, kept with
/// that answer and the screenshot's capture time so they are drawn only
/// beside that answer and on that screenshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuideMarks {
    pub answer: String,
    pub screenshot: std::time::Instant,
    pub marks: Vec<Mark>,
}

/// What follows "Marks:" on a line, ignoring Markdown emphasis around it.
fn marks_line(line: &str) -> Option<&str> {
    let line = line.trim().trim_start_matches(['*', '_', '`']);
    line.get(..6)
        .filter(|label| label.eq_ignore_ascii_case("marks:"))
        .map(|_| &line[6..])
}

/// Splits a Guide me answer into the text a person reads and hears, and the
/// marks on its Marks line, in the pixels of the image the model saw (`sent`).
/// Marks outside that image, beyond step three or repeating a step are
/// dropped; an unreadable Marks line is still removed and gives none.
pub fn split_marks(reply: &str, sent: (u32, u32)) -> (String, Vec<Mark>) {
    let lines: Vec<&str> = reply.lines().collect();
    let Some(at) = lines.iter().rposition(|line| marks_line(line).is_some()) else {
        return (reply.to_owned(), Vec::new());
    };
    let mut marks: Vec<Mark> = Vec::new();
    for entry in marks_line(lines[at]).unwrap_or_default().split(';') {
        let numbers: Vec<u32> = entry
            .split(|c: char| !c.is_ascii_digit())
            .filter_map(|n| n.parse().ok())
            .collect();
        if let [step, x, y] = numbers[..]
            && (1..=MAX_STEPS).contains(&step)
            && x < sent.0
            && y < sent.1
            && !marks.iter().any(|m| m.step == step)
        {
            marks.push(Mark { step, x, y });
        }
    }
    let text = lines
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != at)
        .map(|(_, line)| *line)
        .collect::<Vec<_>>()
        .join("\n");
    (text.trim_end().to_owned(), marks)
}

/// A partly streamed answer without a Marks line that has started to arrive,
/// so the line never flashes on screen before it is removed.
pub fn hide_partial_marks(partial: &str) -> &str {
    let start = partial.trim_end().rfind('\n').map_or(0, |at| at + 1);
    let last = partial[start..].trim().trim_start_matches(['*', '_', '`']);
    let lower = last.to_ascii_lowercase();
    let started =
        !lower.is_empty() && ("marks:".starts_with(&lower) || lower.starts_with("marks:"));
    if started {
        partial[..start].trim_end()
    } else {
        partial
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SENT: (u32, u32) = (1600, 900);

    fn mark(step: u32, x: u32, y: u32) -> Mark {
        Mark { step, x, y }
    }

    #[test]
    fn the_marks_line_is_removed_and_read() {
        let reply = "Your file isn't saved.\n1. Click **File**.\n2. Choose **Save**.\n\nMarks: 1 (12, 8); 2 (40, 96)";
        let (text, marks) = split_marks(reply, SENT);
        assert_eq!(
            text,
            "Your file isn't saved.\n1. Click **File**.\n2. Choose **Save**."
        );
        assert_eq!(marks, [mark(1, 12, 8), mark(2, 40, 96)]);
    }

    #[test]
    fn emphasis_case_and_none_are_understood() {
        let (text, marks) = split_marks("Look at the top.\n**marks:** none", SENT);
        assert_eq!(text, "Look at the top.");
        assert!(marks.is_empty());
        let (_, marks) = split_marks("Step.\n**Marks:** 3 (5, 6)", SENT);
        assert_eq!(marks, [mark(3, 5, 6)]);
    }

    #[test]
    fn marks_that_cannot_be_right_are_dropped() {
        let reply =
            "Steps.\nMarks: 1 (1600, 5); 2 (5, 900); 4 (1, 1); 3 (7, 7); 3 (8, 8); 1 (2); junk";
        let (text, marks) = split_marks(reply, SENT);
        assert_eq!(text, "Steps.");
        // Off the picture, step four, a repeated step and malformed entries go.
        assert_eq!(marks, [mark(3, 7, 7)]);
    }

    #[test]
    fn an_answer_without_marks_is_unchanged() {
        let reply = "Click the remarkable **Save** button.\nThen check the marks: none here.";
        assert_eq!(split_marks(reply, SENT), (reply.to_owned(), Vec::new()));
    }

    #[test]
    fn a_marks_line_is_hidden_while_it_streams_in() {
        let shown = "1. Click Save.";
        assert_eq!(hide_partial_marks("1. Click Save.\nMa"), shown);
        assert_eq!(hide_partial_marks("1. Click Save.\n**Marks: 1 (4"), shown);
        assert_eq!(hide_partial_marks(shown), shown);
        let step = "1. Click Save.\nMake sure";
        assert_eq!(hide_partial_marks(step), step);
    }
}
