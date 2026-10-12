//! Opt-in desktop capabilities for Windows, macOS and Linux. All operations
//! block: run them on a worker thread. Screen images and audio are never kept;
//! where the OS tool can only write a file, the capture passes through a private
//! temporary file that is deleted immediately. A cancellation handle terminates
//! the helper process (including speech playback/recognition). Input executes
//! exactly one reviewed action; callers must never approve actions automatically.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

#[cfg_attr(not(unix), allow(dead_code))]
mod listen;
#[cfg_attr(windows, path = "windows.rs")]
#[cfg_attr(unix, path = "unix.rs")]
#[cfg_attr(not(any(windows, unix)), path = "unsupported.rs")]
mod sys;
#[cfg(unix)]
mod whisper;

/// Where listening finds whisper.cpp. Empty fields mean "find automatically".
/// Windows uses its built-in recognizer and ignores these settings.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VoiceConfig {
    pub whisper_program: Option<PathBuf>,
    pub whisper_model: Option<PathBuf>,
}

/// Whether listening (Talk and Dictate) can run, with a human-readable reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Listening {
    Ready(String),
    Unavailable(String),
}
impl Listening {
    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ready(_))
    }
    pub fn describe(&self) -> &str {
        match self {
            Self::Ready(s) | Self::Unavailable(s) => s,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Cancellation(Arc<AtomicBool>);
impl Cancellation {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The capability does not exist on this platform yet; the text says what does.
    Unsupported(&'static str),
    Cancelled,
    InvalidInput(String),
    Failed(String),
    TimedOut,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(s) => f.write_str(s),
            Self::Cancelled => write!(f, "Operation stopped"),
            Self::TimedOut => write!(f, "Operation timed out"),
            Self::InvalidInput(s) | Self::Failed(s) => f.write_str(s),
        }
    }
}
impl std::error::Error for Error {}

#[derive(Debug)]
pub struct Screenshot {
    pub png: Vec<u8>,
    pub width: u32,
    pub height: u32,
    /// The window in front while the screenshot was taken, where the OS can
    /// say (Windows). James is minimized during capture, so this is the app
    /// the person chose to show.
    pub target: Option<Target>,
}

/// One top-level window, as Windows identified it at capture time. Input is
/// only sent while this same window is in front.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Target {
    /// The window handle, as a number.
    pub window: i64,
    pub process_id: u32,
    pub process_name: String,
    pub title: String,
}

impl Target {
    /// Keeps OS-supplied names printable and short before they are shown or stored.
    pub fn sanitized(mut self) -> Self {
        let clean = |text: &str, max: usize| -> String {
            text.chars()
                .map(|c| if c.is_control() { ' ' } else { c })
                .take(max)
                .collect::<String>()
                .trim()
                .to_owned()
        };
        self.process_name = clean(&self.process_name, 64);
        self.title = clean(&self.title, 120);
        self
    }

    /// How the window is named to a person: title and program.
    pub fn describe(&self) -> String {
        match (self.title.is_empty(), self.process_name.is_empty()) {
            (false, false) => format!("“{}” ({})", self.title, self.process_name),
            (false, true) => format!("“{}”", self.title),
            (true, false) => format!("a {} window", self.process_name),
            (true, true) => "an untitled window".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Key {
    Enter,
    Tab,
    Escape,
    Backspace,
    Delete,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    Space,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    Click { x: u32, y: u32 },
    Key { key: Key },
    TypeText { text: String },
}

impl Action {
    pub fn validate(&self, width: u32, height: u32) -> Result<(), Error> {
        if width == 0 || height == 0 || width > 32768 || height > 32768 {
            return Err(Error::InvalidInput(
                "Invalid primary screen dimensions".into(),
            ));
        }
        match self {
            Self::Click { x, y } if *x >= width || *y >= height => Err(Error::InvalidInput("Click is outside the primary screen".into())),
            Self::TypeText { text } if text.is_empty() || text.chars().count() > 1000 || text.chars().any(char::is_control) => Err(Error::InvalidInput("Type 1–1000 printable characters; use a separate reviewed key action for Enter or Tab".into())),
            _ => Ok(()),
        }
    }
}

/// One-use permission containing the exact immutable action shown to the user.
/// Construct ONLY in response to an explicit user confirmation in the review UI.
#[derive(Debug)]
pub struct ReviewedAction {
    action: Action,
    width: u32,
    height: u32,
    target: Option<Target>,
}
impl ReviewedAction {
    /// `target` is the window the screenshot showed in front. When present,
    /// the input helper refuses to send unless that window is in front.
    pub fn approve(
        action: Action,
        width: u32,
        height: u32,
        target: Option<Target>,
    ) -> Result<Self, Error> {
        action.validate(width, height)?;
        if target
            .as_ref()
            .is_some_and(|t| t.process_id == std::process::id())
        {
            return Err(Error::InvalidInput(
                "The screenshot shows James itself in front. Capture the app you want to act in."
                    .into(),
            ));
        }
        Ok(Self {
            action,
            width,
            height,
            target,
        })
    }
    pub fn action(&self) -> &Action {
        &self.action
    }
    pub fn target(&self) -> Option<&Target> {
        self.target.as_ref()
    }
    pub fn screen(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

/// Checks, without touching the microphone, whether listening can run.
/// Only fast filesystem checks are made, so this is safe to call from a UI.
pub fn listening(config: &VoiceConfig) -> Listening {
    sys::listening(config)
}

/// True where listening uses whisper.cpp and therefore needs setup (macOS, Linux).
pub fn uses_whisper() -> bool {
    sys::USES_WHISPER
}

/// The folder James searches for whisper.cpp models (`ggml-*.bin`), if any.
pub fn whisper_models_dir() -> Option<PathBuf> {
    sys::whisper_models_dir()
}

/// Whether reviewed clicks, keys and text can be sent on this platform.
pub fn can_control_input() -> bool {
    sys::CAN_CONTROL_INPUT
}

/// One line describing voice support on this platform, for settings screens.
pub fn voice_support() -> &'static str {
    sys::VOICE_SUPPORT
}

/// Captures the primary monitor (or the whole desktop where the OS tool cannot
/// select one) as PNG bytes. Must be initiated by the user.
pub fn capture_screen(cancel: &Cancellation) -> Result<Screenshot, Error> {
    sys::capture_screen(cancel)
}

/// Recognize one utterance. `timeout` (1–60 seconds) is how long to wait for
/// speech to start; with whisper.cpp the utterance ends when the speaker pauses.
/// Returns an empty string when nothing was said. Stop cancels the helper.
pub fn dictate(
    timeout: Duration,
    config: &VoiceConfig,
    cancel: &Cancellation,
) -> Result<String, Error> {
    if timeout < Duration::from_secs(1) || timeout > Duration::from_secs(60) {
        return Err(Error::InvalidInput(
            "Dictation timeout must be 1–60 seconds".into(),
        ));
    }
    sys::dictate(timeout, config, cancel)
}

pub fn speak(text: &str, cancel: &Cancellation) -> Result<(), Error> {
    if text.trim().is_empty() || text.chars().count() > 8000 {
        return Err(Error::InvalidInput(
            "Speech requires 1–8000 characters".into(),
        ));
    }
    sys::speak(text, cancel)
}

/// Executes exactly one consumed review. Revalidates the live screen
/// dimensions and, where the review names a target window, that this window
/// is in front (and, for a click, under the pointer) immediately before input.
pub fn execute(review: ReviewedAction, cancel: &Cancellation) -> Result<(), Error> {
    review.action.validate(review.width, review.height)?;
    sys::execute(
        &review.action,
        review.width,
        review.height,
        review.target.as_ref(),
        cancel,
    )
}

/// Reads width and height from a PNG's IHDR chunk.
#[cfg_attr(not(unix), allow(dead_code))]
fn png_size(png: &[u8]) -> Option<(u32, u32)> {
    if png.len() < 24 || !png.starts_with(b"\x89PNG\r\n\x1a\n") || &png[12..16] != b"IHDR" {
        return None;
    }
    let width = u32::from_be_bytes(png[16..20].try_into().ok()?);
    let height = u32::from_be_bytes(png[20..24].try_into().ok()?);
    (width > 0 && height > 0).then_some((width, height))
}

/// Runs one helper process with bounded output, a deadline and cancellation.
/// Variable input travels only over stdin (or as discrete arguments chosen by
/// the platform module), never through a shell.
#[cfg(any(windows, unix))]
fn supervise(
    mut command: std::process::Command,
    input: &[u8],
    timeout: Duration,
    cancel: &Cancellation,
) -> Result<Vec<u8>, Error> {
    use std::{
        io::{Read, Write},
        process::Stdio,
        time::Instant,
    };
    if cancel.is_cancelled() {
        return Err(Error::Cancelled);
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| Error::Failed(format!("Could not launch desktop helper: {e}")))?;
    let mut stdin = child.stdin.take().expect("piped stdin");
    if let Err(error) = stdin.write_all(input) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(Error::Failed(error.to_string()));
    }
    drop(stdin);
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let out = std::thread::spawn(move || {
        let mut v = Vec::new();
        stdout
            .take(64 * 1024 * 1024 + 1)
            .read_to_end(&mut v)
            .map(|_| v)
    });
    let err = std::thread::spawn(move || {
        let mut v = Vec::new();
        stderr.take(65537).read_to_end(&mut v).map(|_| v)
    });
    let start = Instant::now();
    let status = loop {
        if cancel.is_cancelled() || start.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            let _ = out.join();
            let _ = err.join();
            return Err(if cancel.is_cancelled() {
                Error::Cancelled
            } else {
                Error::TimedOut
            });
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => std::thread::sleep(Duration::from_millis(40)),
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = out.join();
                let _ = err.join();
                return Err(Error::Failed(e.to_string()));
            }
        }
    };
    let output = out
        .join()
        .map_err(|_| Error::Failed("Output reader failed".into()))?
        .map_err(|e| Error::Failed(e.to_string()))?;
    let error = err
        .join()
        .map_err(|_| Error::Failed("Error reader failed".into()))?
        .map_err(|e| Error::Failed(e.to_string()))?;
    if output.len() > 64 * 1024 * 1024 || error.len() > 65536 {
        return Err(Error::Failed(
            "Desktop helper output exceeded its limit".into(),
        ));
    }
    if !status.success() {
        return Err(Error::Failed(
            String::from_utf8_lossy(&error).trim().to_owned(),
        ));
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clicks_must_be_within_screen() {
        assert!(
            ReviewedAction::approve(Action::Click { x: 1919, y: 1079 }, 1920, 1080, None).is_ok()
        );
        assert!(
            ReviewedAction::approve(Action::Click { x: 1920, y: 1079 }, 1920, 1080, None).is_err()
        );
        assert!(ReviewedAction::approve(Action::Click { x: 0, y: 0 }, 0, 1080, None).is_err());
    }

    fn target(process_id: u32) -> Target {
        Target {
            window: 0x1234,
            process_id,
            process_name: "notepad".into(),
            title: "Untitled - Notepad".into(),
        }
    }

    #[test]
    fn james_itself_is_never_the_target() {
        let tab = || Action::Key { key: Key::Tab };
        assert!(
            ReviewedAction::approve(tab(), 1920, 1080, Some(target(std::process::id()))).is_err()
        );
        let review =
            ReviewedAction::approve(tab(), 1920, 1080, Some(target(std::process::id() + 1)))
                .unwrap();
        assert_eq!(review.target().unwrap().process_name, "notepad");
        assert_eq!(review.screen(), (1920, 1080));
    }

    #[test]
    fn window_names_are_printable_and_bounded() {
        let named = Target {
            title: format!("Evil\u{1b}[2J{}", "x".repeat(500)),
            process_name: "app\nname".into(),
            ..target(1)
        }
        .sanitized();
        assert!(!named.title.chars().any(char::is_control));
        assert_eq!(named.title.chars().count(), 120);
        assert_eq!(named.process_name, "app name");
        assert_eq!(target(1).describe(), "“Untitled - Notepad” (notepad)");
        let untitled = Target {
            title: String::new(),
            ..target(1)
        };
        assert_eq!(untitled.describe(), "a notepad window");
    }
    #[test]
    fn text_cannot_submit_or_escape_review() {
        for text in ["hello\n", "hello\t", "\0", ""] {
            assert!(
                Action::TypeText { text: text.into() }
                    .validate(1920, 1080)
                    .is_err()
            );
        }
        assert!(
            Action::TypeText {
                text: "{ENTER}+^% café".into()
            }
            .validate(1920, 1080)
            .is_ok()
        );
        assert!(
            Action::TypeText {
                text: "a".repeat(1001)
            }
            .validate(1920, 1080)
            .is_err()
        );
    }
    #[test]
    fn action_deserialization_rejects_shell_and_chords() {
        assert!(serde_json::from_str::<Action>(r#"{"kind":"shell","command":"whoami"}"#).is_err());
        assert!(serde_json::from_str::<Action>(r#"{"kind":"key","key":"ctrl_r"}"#).is_err());
        assert!(
            serde_json::from_str::<Action>(r#"{"kind":"key","key":"enter","command":"whoami"}"#)
                .is_err()
        );
    }
    #[test]
    fn cancellation_shared_between_threads() {
        let cancel = Cancellation::default();
        let other = cancel.clone();
        std::thread::spawn(move || other.cancel()).join().unwrap();
        assert!(cancel.is_cancelled());
    }
    #[test]
    fn invalid_voice_inputs_do_not_launch_helpers() {
        assert!(matches!(
            dictate(
                Duration::ZERO,
                &VoiceConfig::default(),
                &Cancellation::default()
            ),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            speak("", &Cancellation::default()),
            Err(Error::InvalidInput(_))
        ));
    }
    #[test]
    fn png_dimensions_come_from_the_header() {
        let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
        png.extend_from_slice(&1920_u32.to_be_bytes());
        png.extend_from_slice(&1080_u32.to_be_bytes());
        assert_eq!(png_size(&png), Some((1920, 1080)));
        assert_eq!(png_size(&png[..20]), None);
        assert_eq!(png_size(b"not a png at all, definitely not"), None);
        let mut empty = png.clone();
        empty[16..20].copy_from_slice(&0_u32.to_be_bytes());
        assert_eq!(png_size(&empty), None);
    }
    #[test]
    fn platform_reports_its_capabilities() {
        assert!(!voice_support().is_empty());
        let status = listening(&VoiceConfig::default());
        assert!(!status.describe().is_empty());
        if cfg!(windows) {
            assert!(status.is_ready() && can_control_input() && !uses_whisper());
        }
        if cfg!(unix) {
            assert!(uses_whisper() && whisper_models_dir().is_some_and(|d| d.is_absolute()));
        }
    }
    #[test]
    fn explicit_whisper_paths_must_exist() {
        let config = VoiceConfig {
            whisper_program: Some(PathBuf::from("/definitely/not/whisper-cli")),
            whisper_model: None,
        };
        if uses_whisper() {
            assert!(!listening(&config).is_ready());
        }
    }
    #[cfg(any(windows, unix))]
    #[test]
    #[ignore = "Captures the screen in memory; run only with user authorization"]
    fn desktop_capture_smoke() {
        let screen = capture_screen(&Cancellation::default()).expect("screen capture");
        assert!(screen.width > 0 && screen.height > 0);
        assert!(screen.png.starts_with(b"\x89PNG\r\n\x1a\n"));
    }
    #[cfg(any(windows, unix))]
    #[test]
    #[ignore = "Captures the screen in memory; run only with user authorization"]
    fn desktop_capture_names_the_window_in_front() {
        let screen = capture_screen(&Cancellation::default()).expect("screen capture");
        if cfg!(windows) {
            // A desktop session always has some window in front, even if it is the shell.
            let target = screen.target.expect("the window in front");
            assert!(target.window != 0 && target.process_id != 0, "{target:?}");
            println!("In front at capture: {}", target.describe());
        } else {
            assert!(screen.target.is_none());
        }
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "Runs the Windows input helper; it must refuse before sending anything"]
    fn windows_input_refuses_when_the_target_is_not_in_front() {
        // A window handle no real window has: the helper must refuse before input.
        let review = ReviewedAction::approve(
            Action::Key { key: Key::Escape },
            1,
            1,
            Some(Target {
                window: 0x7fff_fff1,
                process_id: u32::MAX - 1,
                process_name: "nobody".into(),
                title: "No such window".into(),
            }),
        )
        .unwrap();
        // The primary screen is not 1x1, so the size check also refuses; use the
        // real size so the target check is what decides.
        let screen = capture_screen(&Cancellation::default()).expect("screen capture");
        let review = ReviewedAction {
            width: screen.width,
            height: screen.height,
            ..review
        };
        let error = execute(review, &Cancellation::default())
            .unwrap_err()
            .to_string();
        assert!(error.contains("nothing was sent"), "{error}");
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "Starts Windows speech helpers; run explicitly on a desktop"]
    fn windows_voice_cancel_smoke() {
        for dictation in [false, true] {
            let cancel = Cancellation::default();
            let stop = cancel.clone();
            let worker = std::thread::spawn(move || {
                if dictation {
                    dictate(Duration::from_secs(60), &VoiceConfig::default(), &cancel).map(|_| ())
                } else {
                    speak(&"Speech cancellation check. ".repeat(200), &cancel)
                }
            });
            std::thread::sleep(Duration::from_millis(500));
            stop.cancel();
            assert!(matches!(worker.join().unwrap(), Err(Error::Cancelled)));
        }
    }
}
