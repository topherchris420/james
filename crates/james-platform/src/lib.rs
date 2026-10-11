//! Opt-in Windows capabilities. All operations block: run them on a worker thread.
//! Capture/audio are never written to disk. A cancellation handle terminates the
//! helper process (including speech playback/recognition). Input executes exactly
//! one reviewed action; callers must never approve actions automatically.

use serde::{Deserialize, Serialize};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

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
    Unsupported,
    Cancelled,
    InvalidInput(String),
    Failed(String),
    TimedOut,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported => write!(f, "This capability requires Windows"),
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
}
impl ReviewedAction {
    pub fn approve(action: Action, width: u32, height: u32) -> Result<Self, Error> {
        action.validate(width, height)?;
        Ok(Self {
            action,
            width,
            height,
        })
    }
    pub fn action(&self) -> &Action {
        &self.action
    }
}

pub fn capture_screen(cancel: &Cancellation) -> Result<Screenshot, Error> {
    use base64::Engine;
    #[derive(Deserialize)]
    struct Response {
        png: String,
        width: u32,
        height: u32,
    }
    let value = run(
        include_str!("capture.ps1"),
        &serde_json::json!({}),
        Duration::from_secs(20),
        cancel,
    )?;
    let response: Response =
        serde_json::from_slice(&value).map_err(|e| Error::Failed(e.to_string()))?;
    let png = base64::engine::general_purpose::STANDARD
        .decode(response.png)
        .map_err(|e| Error::Failed(e.to_string()))?;
    Ok(Screenshot {
        png,
        width: response.width,
        height: response.height,
    })
}

/// Recognize one utterance, bounded to 1–60 seconds. Requires an installed
/// Windows speech recognizer and microphone permission. Stop cancels the helper.
pub fn dictate(timeout: Duration, cancel: &Cancellation) -> Result<String, Error> {
    if timeout < Duration::from_secs(1) || timeout > Duration::from_secs(60) {
        return Err(Error::InvalidInput(
            "Dictation timeout must be 1–60 seconds".into(),
        ));
    }
    let value = run(
        include_str!("dictate.ps1"),
        &serde_json::json!({"seconds": timeout.as_secs_f64()}),
        timeout + Duration::from_secs(10),
        cancel,
    )?;
    serde_json::from_slice(&value).map_err(|e| Error::Failed(e.to_string()))
}

pub fn speak(text: &str, cancel: &Cancellation) -> Result<(), Error> {
    if text.trim().is_empty() || text.chars().count() > 8000 {
        return Err(Error::InvalidInput(
            "Speech requires 1–8000 characters".into(),
        ));
    }
    run(
        include_str!("speak.ps1"),
        &serde_json::json!({"text": text}),
        Duration::from_secs(600),
        cancel,
    )
    .map(|_| ())
}

/// Executes exactly one consumed review. Revalidates the live screen dimensions.
/// The target application must be foreground when this is invoked.
pub fn execute(review: ReviewedAction, cancel: &Cancellation) -> Result<(), Error> {
    review.action.validate(review.width, review.height)?;
    run(include_str!("input.ps1"), &serde_json::json!({"action": review.action, "width": review.width, "height": review.height}), Duration::from_secs(15), cancel).map(|_| ())
}

#[cfg(not(windows))]
fn run(_: &str, _: &serde_json::Value, _: Duration, _: &Cancellation) -> Result<Vec<u8>, Error> {
    Err(Error::Unsupported)
}

#[cfg(windows)]
fn run(
    script: &str,
    input: &serde_json::Value,
    timeout: Duration,
    cancel: &Cancellation,
) -> Result<Vec<u8>, Error> {
    use base64::Engine;
    use std::{
        io::{Read, Write},
        os::windows::process::CommandExt,
        process::{Command, Stdio},
        time::Instant,
    };
    if cancel.is_cancelled() {
        return Err(Error::Cancelled);
    }
    let encoded = base64::engine::general_purpose::STANDARD.encode(
        script
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>(),
    );
    let system_root = std::env::var_os("SystemRoot")
        .ok_or_else(|| Error::Failed("SystemRoot is not configured".into()))?;
    let powershell = std::path::PathBuf::from(system_root)
        .join("System32/WindowsPowerShell/v1.0/powershell.exe");
    if !powershell.is_absolute() {
        return Err(Error::Failed("SystemRoot must be absolute".into()));
    }
    let mut child = Command::new(powershell)
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-EncodedCommand",
            &encoded,
        ])
        .creation_flags(0x08000000)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| Error::Failed(format!("Could not launch Windows capability: {e}")))?;
    let mut stdin = child.stdin.take().expect("piped stdin");
    let bytes = serde_json::to_vec(input).map_err(|e| Error::Failed(e.to_string()))?;
    if let Err(error) = stdin.write_all(&bytes) {
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
            "Windows helper output exceeded its limit".into(),
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
        assert!(ReviewedAction::approve(Action::Click { x: 1919, y: 1079 }, 1920, 1080).is_ok());
        assert!(ReviewedAction::approve(Action::Click { x: 1920, y: 1079 }, 1920, 1080).is_err());
        assert!(ReviewedAction::approve(Action::Click { x: 0, y: 0 }, 0, 1080).is_err());
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
            dictate(Duration::ZERO, &Cancellation::default()),
            Err(Error::InvalidInput(_))
        ));
        assert!(matches!(
            speak("", &Cancellation::default()),
            Err(Error::InvalidInput(_))
        ));
    }
    #[cfg(windows)]
    #[test]
    #[ignore = "Captures the primary display in memory; run only with user authorization"]
    fn windows_capture_smoke() {
        let screen = capture_screen(&Cancellation::default()).expect("screen capture");
        assert!(screen.width > 0 && screen.height > 0);
        assert!(screen.png.starts_with(b"\x89PNG\r\n\x1a\n"));
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
                    dictate(Duration::from_secs(60), &cancel).map(|_| ())
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
