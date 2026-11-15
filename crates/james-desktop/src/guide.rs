//! Guide me: an ordered checklist read from the model's answer, with where on
//! the screenshot each step's control is. The model adds one machine-readable
//! line to its answer; James removes it before the answer is shown, saved or
//! spoken, and draws numbered marks on its own copy of the screenshot. Marks
//! only point: they never act, and an optional, click-through desktop overlay
//! can show the same visible marks. Progress through the checklist is the
//! user's own confirmation, kept in memory for the latest answer alone.

use crate::theme;
use eframe::egui::{self, Frame, RichText};

/// Asks for a short numbered plan and the Marks line, after the attached
/// screenshot's dimensions.
pub const MARKS_CONTEXT: &str = "Give at most three steps as a numbered list, one instruction per line, and keep any explanation around the list. After the steps, add one last line in exactly this form, giving where each step's button, field or link is in the attached screenshot, in pixels: Marks: 1 (x, y); 2 (x, y). Leave out a step that has nothing on screen to point at, and write Marks: none if no step has one. James removes this line before showing or reading the answer.";

/// The most steps Guide me suggests, and so the most marks.
pub const MAX_STEPS: u32 = 3;

/// Where the user has got to, confirmed by them and never inferred. Hiding a
/// number changes nothing here, and neither does the app looking different.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Progress {
    Underway,
    Paused,
    Finished,
    Ended,
}

/// One instruction from the latest answer's numbered list, kept whole even when
/// it spans several lines. Whether it has somewhere to point is separate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub number: u32,
    pub text: String,
}

/// How many steps the user has explicitly ticked off.
type Done = u8;

/// The checklist for one Guide me answer. Text progress lives here so it
/// exists even when the model gave no usable coordinates (Marks: none).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuidePlan {
    /// A stable identity for the answer this plan belongs to.
    pub answer: String,
    pub screenshot: std::time::Instant,
    pub steps: Vec<Step>,
    pub marks: Vec<Mark>,
    mode: Progress,
    done: Done,
    /// Which numbers the user has hidden; presentation only.
    hidden: u8,
}

/// What the shared controls were asked to do this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Choices {
    pub changed: bool,
    pub advance: bool,
    pub pause: bool,
    pub resume: bool,
    pub end: bool,
    /// Ask for an advisory check of the last ticked-off step: a fresh
    /// capture and Guide me question, never an automatic tick or action.
    pub check: bool,
}

impl GuidePlan {
    pub fn new(
        answer: String,
        screenshot: std::time::Instant,
        steps: Vec<Step>,
        marks: Vec<Mark>,
    ) -> Self {
        let steps: Vec<Step> = steps
            .into_iter()
            .filter(|step| (1..=MAX_STEPS).contains(&step.number) && !step.text.trim().is_empty())
            .take(MAX_STEPS as usize)
            .collect();
        let marks = marks
            .into_iter()
            .filter(|mark| {
                // With steps, a mark must point at one of them. Without any
                // parsed steps, usable marks are kept as a numbers-only plan;
                // no step text or progress is invented for them.
                steps.is_empty() || steps.iter().any(|step| step.number == mark.step)
            })
            .collect();
        Self {
            answer,
            screenshot,
            steps,
            marks,
            mode: Progress::Underway,
            done: 0,
            hidden: 0,
        }
    }

    /// The steps the user has explicitly ticked off, in tick order.
    pub fn done_steps(&self) -> &[Step] {
        &self.steps[..self.done as usize]
    }

    /// How many steps the user have explicitly ticked off.
    pub fn done_count(&self) -> usize {
        self.done as usize
    }

    #[cfg(test)]
    pub fn mode(&self) -> Progress {
        self.mode
    }

    /// The step the user is on, while the plan is still running.
    pub fn current(&self) -> Option<u32> {
        (self.mode == Progress::Underway || self.mode == Progress::Paused)
            .then(|| 1 + u32::from(self.done))
            .filter(|number| *number <= MAX_STEPS)
            .filter(|number| self.steps.iter().any(|step| step.number == *number))
    }

    pub fn step(&self, number: u32) -> Option<&Step> {
        self.steps.iter().find(|step| step.number == number)
    }

    /// The current instruction, bounded for a one-line overlay label.
    pub fn current_label(&self, limit: usize) -> Option<String> {
        let text = self.current().and_then(|n| self.step(n)).map(|s| &s.text)?;
        let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
        if flat.chars().count() <= limit {
            return Some(flat);
        }
        let mut cut: String = flat.chars().take(limit.saturating_sub(1)).collect();
        cut.push('…');
        Some(cut)
    }

    /// Whether the user's confirmation of the current step can be accepted.
    /// A paused, finished or ended plan refuses it; so does a plan with no
    /// steps at all.
    pub fn can_advance(&self) -> bool {
        self.mode == Progress::Underway && self.current().is_some()
    }

    /// Records the user's own confirmation. The last step finishes the plan.
    /// Never executes the step, proposes an action, or contacts a model.
    pub fn advance(&mut self) -> bool {
        if !self.can_advance() {
            return false;
        }
        self.done = self.done.saturating_add(1).min(self.steps.len() as u8);
        if self.done as usize >= self.steps.len() {
            self.mode = Progress::Finished;
        }
        true
    }

    pub fn pause(&mut self) -> bool {
        if self.mode != Progress::Underway {
            return false;
        }
        self.mode = Progress::Paused;
        true
    }

    /// Only resumes a paused plan, and only restores the current step's number
    /// if its own screenshot is still the one on screen (`current`). This says
    /// nothing about what the desktop actually looks like.
    pub fn resume(&mut self, same_screenshot: bool) -> bool {
        if self.mode != Progress::Paused {
            return false;
        }
        self.mode = Progress::Underway;
        if same_screenshot && let Some(number) = self.current() {
            self.show(number);
        }
        true
    }

    pub fn end(&mut self) -> bool {
        if self.mode == Progress::Ended {
            return false;
        }
        self.mode = Progress::Ended;
        true
    }

    /// Whether desktop numbers should be drawn at all.
    pub fn numbers_running(&self) -> bool {
        self.mode == Progress::Underway
    }

    pub fn is_visible(&self, step: u32) -> bool {
        (1..=MAX_STEPS).contains(&step)
            && self.marks.iter().any(|mark| mark.step == step)
            && self.hidden & (1 << (step - 1)) == 0
    }

    /// Visibility is presentation only. Hiding a mark does not complete a step,
    /// remove its instruction, or authorize any action.
    pub fn set_visible(&mut self, step: u32, visible: bool) -> bool {
        if !(1..=MAX_STEPS).contains(&step) || !self.marks.iter().any(|m| m.step == step) {
            return false;
        }
        let before = self.hidden;
        let bit = 1 << (step - 1);
        if visible {
            self.hidden &= !bit;
        } else {
            self.hidden |= bit;
        }
        before != self.hidden
    }

    fn show(&mut self, step: u32) {
        if (1..=MAX_STEPS).contains(&step) {
            self.hidden &= !(1 << (step - 1));
        }
    }

    pub fn show_all(&mut self) -> bool {
        let changed = self.hidden != 0;
        self.hidden = 0;
        changed
    }

    pub fn all_hidden(&self) -> bool {
        !self.marks.is_empty() && self.marks.iter().all(|m| !self.is_visible(m.step))
    }

    pub fn visible_marks(&self) -> Vec<Mark> {
        if !self.numbers_running() {
            return Vec::new();
        }
        self.marks
            .iter()
            .copied()
            .filter(|mark| self.is_visible(mark.step))
            .collect()
    }

    /// Numbers to place in the desktop overlay: only while running, and the
    /// running step carries the bounded instruction text as its label.
    pub fn desktop_marks(&self) -> Vec<(Mark, String)> {
        if !self.numbers_running() {
            return Vec::new();
        }
        let current = self.current();
        self.visible_marks()
            .into_iter()
            .map(|mark| {
                let label = if Some(mark.step) == current {
                    self.current_label(80).unwrap_or_default()
                } else {
                    String::new()
                };
                (mark, label)
            })
            .collect()
    }
}

/// One row of the checklist as it reads on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    Completed(u32),
    Current(u32),
    Upcoming(u32),
    Stopped(u32),
}

/// The rows to list, following what the user confirmed rather than what the
/// desktop shows.
pub fn rows(plan: &GuidePlan) -> Vec<Row> {
    let current = plan.current();
    plan.steps
        .iter()
        .map(|step| {
            if plan.mode == Progress::Finished {
                Row::Completed(step.number)
            } else {
                match current {
                    Some(at) if step.number < at => Row::Completed(step.number),
                    Some(at) if step.number == at && plan.mode == Progress::Underway => {
                        Row::Current(step.number)
                    }
                    Some(at) if step.number == at => Row::Stopped(step.number),
                    Some(_) => Row::Upcoming(step.number),
                    None => {
                        if plan.mode == Progress::Ended && step.number <= u32::from(plan.done) {
                            Row::Completed(step.number)
                        } else if plan.mode == Progress::Paused {
                            Row::Upcoming(step.number)
                        } else {
                            Row::Stopped(step.number)
                        }
                    }
                }
            }
        })
        .collect()
}

/// A short sentence naming the plan's state. Pure, so it can be checked.
pub fn state_line(plan: &GuidePlan) -> String {
    let total = plan.steps.len();
    let done = usize::from(plan.done);
    if total == 0 {
        return match plan.mode {
            Progress::Underway => {
                "No parsed steps. The model's numbers below are all the guidance there is."
            }
            Progress::Paused => {
                "Guide paused. Its numbers are hidden; no step progress is recorded."
            }
            Progress::Finished => {
                "Guide stopped. No parsed steps were available to confirm completion."
            }
            Progress::Ended => "Guide ended. No step progress was recorded.",
        }
        .to_owned();
    }
    match plan.mode {
        Progress::Underway => format!(
            "Step {} of {total}. Your model's order; tick each one off when you have done it.",
            u32::from(plan.done) + 1
        ),
        Progress::Paused => format!(
            "Paused at step {} of {total}. Nothing advances until you resume.",
            u32::from(plan.done) + 1
        ),
        Progress::Finished => format!(
            "Guide finished: you ticked off all {total} steps. This is your confirmation, not a check of the app."
        ),
        Progress::Ended => format!(
            "Guide ended after {done} of {total} steps. Your drawn notes are still on the screenshot."
        ),
    }
}

/// A compact checklist and one set of controls, shared by the conversation
/// preview and the Screen context. It only reports choices: the caller keeps
/// the plan and decides when to redraw the overlay.
pub fn checklist_ui(plan: &mut GuidePlan, ui: &mut egui::Ui) -> Choices {
    let mut choices = Choices::default();
    ui.add_space(4.0);
    Frame::group(ui.style()).show(ui, |ui| {
        for row in rows(plan) {
            let number = match row {
                Row::Completed(n) | Row::Current(n) | Row::Upcoming(n) | Row::Stopped(n) => n,
            };
            let Some(step) = plan.step(number) else {
                continue;
            };
            // James's fonts have no glyph for a check mark or a small
            // triangle, so done steps are struck through instead of ticked
            // and the current step uses the large triangle that is covered.
            let (glyph, tone, note, struck) = match row {
                Row::Completed(_) => (
                    None,
                    ui.visuals().weak_text_color(),
                    "done, by your word",
                    true,
                ),
                Row::Current(_) => (Some("▶"), theme::accent(ui), "current", false),
                Row::Upcoming(_) => (
                    Some("·"),
                    ui.visuals().text_color(),
                    "upcoming",
                    false,
                ),
                Row::Stopped(_) => (
                    Some("–"),
                    ui.visuals().weak_text_color(),
                    "not reached",
                    false,
                ),
            };
            ui.horizontal_wrapped(|ui| {
                if let Some(glyph) = glyph {
                    ui.label(RichText::new(glyph).color(tone).strong());
                }
                ui.label(RichText::new(format!("{number}.")).color(tone).strong());
                ui.label(if struck {
                    RichText::new(&step.text).weak().strikethrough()
                } else {
                    RichText::new(&step.text)
                });
                ui.small(note);
            });
        }
        ui.horizontal_wrapped(|ui| match plan.mode {
            Progress::Underway => {
                let last = plan.done as usize + 1 == plan.steps.len();
                let label = if last {
                    "Complete current step (last finishes)"
                } else {
                    "Complete current step"
                };
                if ui
                    .add_enabled(plan.can_advance(), egui::Button::new(label))
                    .on_hover_text("Records that you, not James, did this step. James does not click anything.")
                    .clicked()
                {
                    choices.advance = true;
                }
                if plan.done_count() > 0
                    && ui
                        .button("Check it worked")
                        .on_hover_text("Takes a fresh screenshot and asks your model whether the last step looks done. Advisory only: it never ticks a step, clicks or types.")
                        .clicked()
                {
                    choices.check = true;
                }
                if ui.button("Pause guide").clicked() {
                    choices.pause = true;
                }
                if ui.button("End guide").clicked() {
                    choices.end = true;
                }
            }
            Progress::Paused => {
                if ui.button("Resume guide").clicked() {
                    choices.resume = true;
                }
                let blocked = ui
                    .add_enabled(false, egui::Button::new("Complete current step"))
                    .on_disabled_hover_text("Resume first, or end the guide.");
                if blocked.clicked() {
                    choices.advance = true;
                }
                if ui.button("End guide").clicked() {
                    choices.end = true;
                }
            }
            Progress::Finished | Progress::Ended => {
                if ui
                    .add_enabled(false, egui::Button::new("Pause guide"))
                    .on_disabled_hover_text("The guide is no longer running.")
                    .clicked()
                {
                    choices.pause = true;
                }
                if ui.button("End guide").clicked() && plan.mode != Progress::Ended {
                    choices.end = true;
                }
            }
        });
        ui.small(state_line(plan));
    });
    visibility_controls(plan, ui, &mut choices);
    choices
}

/// Applies this frame's choices to the plan. `same_screenshot` is whether the
/// plan's own capture is still the one on screen. Returns true when the
/// desktop overlay needs synchronizing.
pub fn apply_choices(plan: &mut GuidePlan, choices: Choices, same_screenshot: bool) -> bool {
    let before = (plan.mode, plan.done);
    if choices.advance {
        plan.advance();
    }
    if choices.pause {
        plan.pause();
    }
    if choices.resume {
        plan.resume(same_screenshot);
    }
    if choices.end {
        plan.end();
    }
    let after = (plan.mode, plan.done);
    choices.changed || before != after
}

/// The Show step N and Show all steps controls. Returns the combined choices.
fn visibility_controls(plan: &mut GuidePlan, ui: &mut egui::Ui, choices: &mut Choices) {
    let steps: Vec<_> = plan.marks.iter().map(|mark| mark.step).collect();
    ui.horizontal_wrapped(|ui| {
        for step in steps {
            let mut visible = plan.is_visible(step);
            if ui.checkbox(&mut visible, format!("Show step {step}"))
                .on_hover_text("Show or hide this number. This does not operate the app or mark the task complete.")
                .changed()
            {
                choices.changed |= plan.set_visible(step, visible);
            }
        }
        if ui.add_enabled(plan.hidden != 0, egui::Button::new("Show all steps")).clicked() {
            choices.changed |= plan.show_all();
        }
    });
    if plan.marks.is_empty() {
        ui.small(
            "No step numbers: the model found nothing to point at, so follow the steps above.",
        );
    } else {
        let visible = plan.visible_marks().len();
        if plan.all_hidden() {
            ui.small(
                "All step numbers are hidden. The instructions still apply to this screenshot.",
            );
        } else {
            ui.small(format!(
                "Showing {visible} of {} step numbers.",
                plan.marks.len()
            ));
        }
    }
}

/// Reads the ordered numbered list out of an answer. Continuation lines stay
/// with their instruction; ordinary sentences before, between or after the
/// list are left out of the plan. Numbers restart at 1, must not skip, and a
/// malformed or nonconsecutive list is rejected whole rather than turned into a
/// checklist that pretends to be complete.
pub fn parse_steps(answer: &str) -> Vec<Step> {
    let mut steps: Vec<Step> = Vec::new();
    let mut lines = answer.lines().peekable();
    while let Some(line) = lines.next() {
        let Some((number, first)) = numbered(line) else {
            continue;
        };
        if number != steps.len() as u32 + 1 {
            // A list that does not start at 1 or jumps a number is malformed:
            // none of it is trusted as a checklist.
            if steps.is_empty() {
                continue;
            }
            return Vec::new();
        }
        if number > MAX_STEPS {
            // Never more than three model steps: the rest is ignored, not a
            // malformed list.
            break;
        }
        let mut text = first.to_owned();
        while let Some(next) = lines.peek() {
            if next.trim().is_empty() || numbered(next).is_some() {
                break;
            }
            let extra = next.trim();
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(extra);
            lines.next();
        }
        if !text.trim().is_empty() {
            steps.push(Step {
                number,
                text: text.trim().to_owned(),
            });
        }
    }
    steps
}

/// Reads `1. text`, `2) text` or `**3.** text`. Exactly one numbering separator
/// and following whitespace are required; emphasis around a number must match.
/// Decimal values, versions and prose such as `1password` are not steps.
fn numbered(line: &str) -> Option<(u32, &str)> {
    let line = line.trim_start();
    let emphasis = ["**", "__", "*", "_"]
        .into_iter()
        .find(|prefix| line.starts_with(*prefix));
    let line = if let Some(prefix) = emphasis {
        line.strip_prefix(prefix)?
    } else {
        line
    };
    let digits = line.len() - line.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    if digits == 0 || digits > 2 {
        return None;
    }
    let (number, rest) = line.split_at(digits);
    let rest = rest.strip_prefix('.').or_else(|| rest.strip_prefix(')'))?;
    let rest = if let Some(prefix) = emphasis {
        rest.strip_prefix(prefix)?
    } else {
        rest
    };
    if !rest.chars().next()?.is_whitespace() {
        return None;
    }
    let rest = rest.trim();
    if rest.is_empty() {
        return None;
    }
    Some((number.parse().ok()?, rest))
}

/// One numbered mark on the screenshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mark {
    pub step: u32,
    pub x: u32,
    pub y: u32,
}

/// What follows "Marks:" on a line, ignoring Markdown emphasis around it.
fn marks_line(line: &str) -> Option<&str> {
    let line = line.trim().trim_start_matches(['*', '_', '`']);
    line.get(..6)
        .filter(|label| label.eq_ignore_ascii_case("marks:"))
        .map(|_| &line[6..])
}

/// Read the requested tuple grammar without turning a negative or decimal
/// coordinate into a different, apparently valid position.
fn parse_mark(entry: &str) -> Option<Mark> {
    let entry = entry.trim().trim_matches(['*', '_', '`']).trim();
    let (step, coordinates) = entry.split_once('(')?;
    let (x, y) = coordinates.strip_suffix(')')?.split_once(',')?;
    let number = |value: &str| {
        let value = value.trim();
        if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        value.parse::<u32>().ok()
    };
    Some(Mark {
        step: number(step)?,
        x: number(x)?,
        y: number(y)?,
    })
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
        if let Some(mark) = parse_mark(entry)
            && (1..=MAX_STEPS).contains(&mark.step)
            && mark.x < sent.0
            && mark.y < sent.1
            && !marks.iter().any(|m| m.step == mark.step)
        {
            marks.push(mark);
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

    fn plan(answer: &str, at: std::time::Instant, steps: &[&str], marks: Vec<Mark>) -> GuidePlan {
        let steps = steps
            .iter()
            .enumerate()
            .map(|(index, text)| Step {
                number: index as u32 + 1,
                text: (*text).to_owned(),
            })
            .collect();
        GuidePlan::new(answer.to_owned(), at, steps, marks)
    }

    fn three_steps() -> Vec<Mark> {
        vec![mark(1, 2, 3), mark(2, 4, 5), mark(3, 6, 7)]
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

    #[test]
    fn visibility_is_independent_reversible_and_bound_to_one_answer() {
        let marks = vec![mark(1, 2, 3), mark(3, 5, 7)];
        let at = std::time::Instant::now();
        let steps = ["Open the window.", "Read the title.", "Close it."];
        let mut shown = plan("Instructions", at, &steps, marks.clone());
        assert!(shown.set_visible(1, false));
        assert!(!shown.set_visible(1, false));
        assert_eq!(shown.visible_marks(), [marks[1]]);
        for step in [0, 2, 4, u32::MAX] {
            assert!(!shown.set_visible(step, false));
            assert!(!shown.is_visible(step));
        }
        assert!(shown.set_visible(3, false));
        assert!(shown.visible_marks().is_empty());
        assert!(shown.all_hidden());
        assert_eq!(shown.answer, "Instructions");
        assert_eq!(shown.marks, marks);
        assert_eq!(shown.screenshot, at);
        assert!(shown.set_visible(1, true));
        assert_eq!(shown.visible_marks(), [marks[0]]);
        // Progress survives hiding, and replacement resets it.
        assert_eq!(shown.current(), Some(1));
        let replacement = plan("New instructions", at, &["Start over."], marks.clone());
        assert_eq!(replacement.visible_marks(), [marks[0]]);
        assert_eq!(
            replacement.current(),
            Some(1),
            "a new plan restarts progress"
        );
    }

    #[test]
    fn malformed_coordinates_are_not_reinterpreted_as_valid_points() {
        for tuple in [
            "1 (-2, 3)",
            "-1 (2, 3)",
            "1 (+2, 3)",
            "1 (2, -3)",
            "1 (2.0, 3)",
            "1 (2e0, 3)",
            "1 (2 3)",
            "step 1 at 2, 3",
            "1 (2, 3) extra",
            "1 (2, 3, 4)",
            "1 (4294967296, 3)",
            "1 (２, 3)",
            "1 (NaN, 3)",
        ] {
            let (answer, marks) = split_marks(&format!("Advice.\nMarks: {tuple}"), SENT);
            assert_eq!(answer, "Advice.");
            assert!(marks.is_empty(), "accepted malformed tuple {tuple}");
        }
        assert_eq!(
            split_marks("Advice.\n**Marks:** 1 ( 2 , 3 ); 3 (5, 7)**", SENT).1,
            [mark(1, 2, 3), mark(3, 5, 7)]
        );
    }

    #[test]
    fn ordered_instructions_keep_multiline_text_and_ignore_prose() {
        let answer = "Here is how.\n\n1. Open the **Settings** menu\nand choose Appearance.\n2) Read the first heading.\nA word in between.\n3. Close it.";
        let steps = parse_steps(answer);
        assert_eq!(steps.len(), 3);
        assert_eq!(
            steps[0].text,
            "Open the **Settings** menu\nand choose Appearance."
        );
        assert_eq!(steps[1].number, 2);
        assert_eq!(steps[2].text, "Close it.");
        // The ordinary answer text is untouched by the reading.
        assert!(answer.contains("A word in between."));
    }

    #[test]
    fn unnumbered_or_skipping_lists_make_no_steps() {
        assert!(parse_steps("Just try the menu, then save.").is_empty());
        assert!(parse_steps("- Bullet one\n- Bullet two").is_empty());
        // Must start at 1 and must not jump over a number.
        assert!(parse_steps("2. Second first\n3. Third second").is_empty());
        assert!(parse_steps("1. First\n3. Skipped two").is_empty());
        // Real numbering punctuation is required: prose is not a step.
        assert!(parse_steps("1password is my password\n2. Real step").is_empty());
        assert!(parse_steps("1 File was opened\n2. Real step").is_empty());
        // The documented Markdown prefix still works.
        let emphasized = parse_steps("**1.** Bold step\n2) Paren step");
        assert_eq!(
            emphasized
                .iter()
                .map(|s| s.text.clone())
                .collect::<Vec<_>>(),
            ["Bold step", "Paren step"]
        );
        // Never more than three model steps.
        let four = parse_steps("1. A\n2. B\n3. C\n4. D");
        assert_eq!(four.iter().map(|s| s.number).collect::<Vec<_>>(), [1, 2, 3]);
    }

    #[test]
    fn numbered_steps_require_one_separator_and_whitespace() {
        for line in [
            "1.5 seconds",
            "1.2.3",
            "1..text",
            "1.. text",
            "1)) text",
            "1.) text",
            "1.No space",
            "**1.**No space",
            "**1.__ Mismatched",
            "__1.** Mismatched",
            "1.** Unmatched",
            "**1. Missing close",
        ] {
            assert!(numbered(line).is_none(), "accepted {line:?}");
            assert!(parse_steps(line).is_empty(), "made steps from {line:?}");
        }
        for (line, expected) in [
            ("1. First", (1, "First")),
            ("2) Next", (2, "Next")),
            ("**1.** Bold", (1, "Bold")),
            ("__1)__ Underlined", (1, "Underlined")),
            ("*1.* Single", (1, "Single")),
            ("1.\tTabbed", (1, "Tabbed")),
            ("1. Open **Settings**", (1, "Open **Settings**")),
        ] {
            assert_eq!(numbered(line), Some(expected), "rejected {line:?}");
        }
        let wrapped = parse_steps("**1.** First line\ncontinued instruction\n2) Next");
        assert_eq!(wrapped[0].text, "First line\ncontinued instruction");
        assert_eq!(wrapped[1].text, "Next");
    }

    #[test]
    fn marks_only_status_reports_pause_and_end_without_step_progress() {
        let mut plan = GuidePlan::new(
            "Advice".into(),
            std::time::Instant::now(),
            Vec::new(),
            vec![mark(1, 10, 10)],
        );
        assert!(state_line(&plan).starts_with("No parsed steps."));
        assert!(plan.pause());
        assert_eq!(
            state_line(&plan),
            "Guide paused. Its numbers are hidden; no step progress is recorded."
        );
        assert!(plan.visible_marks().is_empty());
        assert!(!plan.advance());
        assert!(plan.resume(true));
        assert!(state_line(&plan).starts_with("No parsed steps."));
        assert!(plan.end());
        assert_eq!(
            state_line(&plan),
            "Guide ended. No step progress was recorded."
        );
        assert!(plan.visible_marks().is_empty());
        assert!(!plan.advance());
        assert!(rows(&plan).is_empty());
    }

    #[test]
    fn marks_without_any_steps_are_kept_without_inventing_a_checklist() {
        // An old valid marks-only answer keeps its usable marks, with no
        // fabricated step text or progress.
        let reply = "Replacement advice.\nMarks: 1 (10, 10)";
        let (text, marks) = split_marks(reply, SENT);
        assert_eq!(text, "Replacement advice.");
        let at = std::time::Instant::now();
        let plan = GuidePlan::new(text, at, Vec::new(), marks);
        assert_eq!(plan.marks, [mark(1, 10, 10)]);
        assert!(plan.steps.is_empty(), "no steps are invented");
        assert_eq!(plan.current(), None);
        assert!(!plan.can_advance(), "nothing to confirm");
        assert!(plan.is_visible(1), "the number itself is still shown");
        assert_eq!(rows(&plan), Vec::new(), "no rows pretend to be a checklist");
        // Steps without coordinates are still a normal checklist.
        let with_steps = GuidePlan::new(
            "Advice".into(),
            at,
            parse_steps("1. Only step."),
            vec![mark(1, 5, 5), mark(2, 6, 6)],
        );
        assert_eq!(with_steps.marks, [mark(1, 5, 5)], "a mark needs its step");
    }

    #[test]
    fn a_checklist_exists_without_any_coordinates() {
        let answer = "Look at the toolbar.\n1. Open the menu.\n2. Choose Save.\nMarks: none";
        let (text, marks) = split_marks(answer, SENT);
        assert!(marks.is_empty());
        let at = std::time::Instant::now();
        let mut plan = plan(&text, at, &["Open the menu.", "Choose Save."], marks);
        assert_eq!(plan.current(), Some(1));
        assert!(plan.visible_marks().is_empty());
        assert!(plan.desktop_marks().is_empty());
        assert!(plan.advance());
        assert_eq!(plan.current(), Some(2));
        assert!(plan.advance());
        assert_eq!(plan.mode(), Progress::Finished);
        assert_eq!(rows(&plan), [Row::Completed(1), Row::Completed(2)]);
    }

    #[test]
    fn user_advance_finishes_only_after_the_last_step() {
        let at = std::time::Instant::now();
        let mut plan = plan("Advice", at, &["One.", "Two.", "Three."], three_steps());
        assert_eq!(
            rows(&plan),
            [Row::Current(1), Row::Upcoming(2), Row::Upcoming(3)]
        );
        assert!(plan.advance());
        assert_eq!(
            rows(&plan),
            [Row::Completed(1), Row::Current(2), Row::Upcoming(3)]
        );
        assert_eq!(plan.mode(), Progress::Underway);
        assert!(plan.advance());
        assert_eq!(plan.current(), Some(3));
        assert!(plan.advance());
        assert_eq!(plan.mode(), Progress::Finished);
        assert_eq!(plan.current(), None);
        assert!(!plan.can_advance());
        assert!(!plan.advance(), "a finished plan does not advance again");
        assert!(!plan.numbers_running());
        assert!(
            plan.visible_marks().is_empty(),
            "finishing suppresses numbers"
        );
        assert_eq!(plan.marks.len(), 3, "the user's marks are not deleted");
    }

    #[test]
    fn pausing_blocks_advancement_and_resume_restores_only_the_current_number() {
        let at = std::time::Instant::now();
        let mut plan = plan("Advice", at, &["One.", "Two.", "Three."], three_steps());
        assert!(plan.set_visible(2, false));
        assert!(plan.advance());
        assert_eq!(plan.current(), Some(2), "step one is completed first");
        assert!(plan.pause());
        assert!(!plan.pause());
        assert_eq!(plan.mode(), Progress::Paused);
        assert_eq!(rows(&plan)[1], Row::Stopped(2));
        assert!(!plan.can_advance());
        assert!(!plan.advance(), "no advancement while paused");
        assert!(!plan.numbers_running());
        assert!(plan.desktop_marks().is_empty());
        // A different screenshot is now current: nothing is claimed about it.
        assert!(plan.resume(false));
        assert_eq!(plan.mode(), Progress::Underway);
        assert!(
            !plan.is_visible(2),
            "a replaced screenshot restores no number"
        );
        assert!(!plan.resume(true), "resuming twice is refused");
        // Pause again and resume on the same screenshot: the current mark is back.
        assert!(plan.pause());
        assert!(plan.resume(true));
        assert!(plan.is_visible(2));
        assert!(plan.advance());
    }

    #[test]
    fn ending_stops_everything_but_keeps_drawn_notes() {
        let at = std::time::Instant::now();
        let mut plan = plan("Advice", at, &["One.", "Two."], vec![mark(1, 2, 3)]);
        assert!(plan.advance());
        assert!(plan.end());
        assert!(!plan.end(), "ending twice changes nothing");
        assert_eq!(plan.mode(), Progress::Ended);
        assert!(!plan.can_advance());
        assert!(!plan.advance());
        assert_eq!(plan.current(), None);
        assert_eq!(rows(&plan), [Row::Completed(1), Row::Stopped(2)]);
        assert!(plan.visible_marks().is_empty());
        assert_eq!(plan.marks.len(), 1);
        assert!(state_line(&plan).contains("ended"));
        assert!(!plan.resume(true));
    }

    #[test]
    fn progress_is_independent_of_which_numbers_are_shown() {
        let at = std::time::Instant::now();
        let mut plan = plan("Advice", at, &["One.", "Two.", "Three."], three_steps());
        assert!(plan.set_visible(1, false));
        assert!(plan.set_visible(2, false));
        assert!(plan.set_visible(3, false));
        assert_eq!(plan.current(), Some(1), "hiding a number ticks nothing");
        assert_eq!(rows(&plan)[0], Row::Current(1));
        assert!(plan.show_all());
        assert_eq!(plan.current(), Some(1));
        assert!(plan.advance());
        assert_eq!(plan.current(), Some(2));
        assert!(plan.visible_marks().len() == 3);
    }

    #[test]
    fn the_desktop_numbers_label_only_the_running_step() {
        let at = std::time::Instant::now();
        let long = "Point at the toolbar, then at the little arrow beside the title bar and wait";
        let mut plan = plan("Advice", at, &[long, "Two.", "Three."], three_steps());
        let marks = plan.desktop_marks();
        assert_eq!(marks[0].0.step, 1);
        assert!(marks[0].1.starts_with("Point at the toolbar"));
        assert!(marks[0].1.chars().count() <= 80);
        assert!(marks[1].1.is_empty());
        plan.advance();
        let marks = plan.desktop_marks();
        assert!(marks[0].1.is_empty());
        assert_eq!(marks[1].1, "Two.");
    }

    #[test]
    fn choices_apply_once_and_note_changes_for_the_overlay() {
        let at = std::time::Instant::now();
        let mut plan = plan("Advice", at, &["One.", "Two."], three_steps());
        let idle = Choices::default();
        assert!(!apply_choices(&mut plan, idle, true));
        assert!(apply_choices(
            &mut plan,
            Choices {
                pause: true,
                ..idle
            },
            true
        ));
        assert_eq!(plan.mode(), Progress::Paused);
        assert!(apply_choices(
            &mut plan,
            Choices {
                resume: true,
                ..idle
            },
            true
        ));
        assert_eq!(plan.mode(), Progress::Underway);
        assert!(apply_choices(
            &mut plan,
            Choices {
                changed: true,
                ..idle
            },
            true
        ));
    }
}
