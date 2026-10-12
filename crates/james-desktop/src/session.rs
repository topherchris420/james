//! What James keeps on this computer, and how it keeps it private.
//!
//! Everything lives in one folder: `%LOCALAPPDATA%\Vers3Dynamics\James` on
//! Windows, `~/.james` elsewhere. On macOS and Linux the folder is owner-only
//! (0700) and every file James writes is owner-only (0600), so other accounts
//! on the computer cannot read conversations or a pending action. Windows
//! already limits `%LOCALAPPDATA%` to the signed-in user. None of this is
//! encryption: anyone who can sign in as you, or read the disk directly, can
//! read these files. Full-disk encryption (BitLocker, FileVault, LUKS) is what
//! protects them at rest.
use james_models::{ChatMessage, LocalProvider};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const MAX_SAVED_MESSAGES: usize = 80;
pub const MAX_MESSAGE_BYTES: usize = 32_768;
const MAX_SESSION_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeChoice {
    /// Follow the operating system's light or dark setting.
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub theme: ThemeChoice,
    pub high_contrast: bool,
    /// Everything on screen is scaled by this, from 0.8 to 2.0.
    pub text_scale: f32,
    /// No animations, spinners or smooth scrolling.
    pub reduce_motion: bool,
    pub speak_replies: bool,
    /// Guide me numbers its steps on the screenshot. Off by default: the
    /// model places the numbers, and small models often misplace them.
    pub guide_marks: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            theme: ThemeChoice::System,
            high_contrast: false,
            text_scale: 1.0,
            reduce_motion: false,
            speak_replies: false,
            guide_marks: false,
        }
    }
}

impl Preferences {
    pub fn text_scale(&self) -> f32 {
        if self.text_scale.is_finite() {
            self.text_scale.clamp(0.8, 2.0)
        } else {
            1.0
        }
    }
}

/// The first-run steps, in order. Progress is saved after every step so an
/// interrupted setup resumes where it stopped.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SetupStep {
    #[default]
    Welcome,
    ModelApp,
    Connect,
    Test,
    Screen,
    Done,
}

impl SetupStep {
    pub const SHOWN: [Self; 5] = [
        Self::Welcome,
        Self::ModelApp,
        Self::Connect,
        Self::Test,
        Self::Screen,
    ];
    pub fn title(self) -> &'static str {
        match self {
            Self::Welcome => "Welcome",
            Self::ModelApp => "Get a model app",
            Self::Connect => "Connect",
            Self::Test => "Say hello",
            Self::Screen => "Screen and safety",
            Self::Done => "Done",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Setup {
    pub step: SetupStep,
    /// A test message got a real reply from the chosen model.
    pub test_passed: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct SavedSession {
    pub port: u16,
    pub provider: LocalProvider,
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub lab_port: u16,
    pub whisper_program: String,
    pub whisper_model: String,
    /// Off means conversations live in memory only and are gone when James closes.
    pub keep_history: bool,
    pub preferences: Preferences,
    /// Absent in sessions saved before guided setup existed.
    pub setup: Option<Setup>,
    /// The James version that last saved this session; empty in sessions
    /// saved before James recorded it.
    pub last_version: String,
}

impl Default for SavedSession {
    fn default() -> Self {
        Self {
            port: 1234,
            provider: LocalProvider::LmStudio,
            model: String::new(),
            messages: Vec::new(),
            lab_port: 5173,
            whisper_program: String::new(),
            whisper_model: String::new(),
            keep_history: true,
            preferences: Preferences::default(),
            setup: None,
            last_version: String::new(),
        }
    }
}

impl SavedSession {
    /// Setup progress, treating a session from before guided setup that
    /// already has a model as set up: an upgrade never forces the tour.
    pub fn setup(&self) -> Setup {
        self.setup.clone().unwrap_or_else(|| Setup {
            step: if self.model.trim().is_empty() {
                SetupStep::Welcome
            } else {
                SetupStep::Done
            },
            test_passed: false,
        })
    }
}

/// Where James keeps its files. Tests and portable use can override it.
pub fn default_session_path() -> PathBuf {
    if let Some(path) = std::env::var_os("LOCALAPPDATA") {
        PathBuf::from(path).join("Vers3Dynamics/James/session.json")
    } else if let Some(path) = std::env::var_os("HOME") {
        PathBuf::from(path).join(".james/session.json")
    } else {
        PathBuf::from("james-session.json")
    }
}

pub fn intent_path(session: &Path) -> PathBuf {
    session.with_file_name("action-intent.json")
}

pub fn crash_report_path(session: &Path) -> PathBuf {
    session.with_file_name("crash.log")
}

/// Keeps only well-formed, bounded conversation turns, newest last.
pub fn bounded(messages: Vec<ChatMessage>) -> Vec<ChatMessage> {
    let kept: Vec<ChatMessage> = messages
        .into_iter()
        .filter(|m| {
            matches!(m.role.as_str(), "user" | "assistant") && m.content.len() <= MAX_MESSAGE_BYTES
        })
        .collect();
    let skip = kept.len().saturating_sub(MAX_SAVED_MESSAGES);
    kept.into_iter().skip(skip).collect()
}

/// Reads the saved session. A missing, oversized or unreadable file starts
/// fresh rather than failing. Files written by older versions with looser
/// permissions are tightened on the way in.
pub fn load(path: &Path) -> SavedSession {
    if let Some(parent) = path.parent() {
        let _ = tighten(parent, true);
    }
    for file in [
        path.to_path_buf(),
        intent_path(path),
        crash_report_path(path),
    ] {
        let _ = tighten(&file, false);
    }
    let mut saved: SavedSession = std::fs::metadata(path)
        .ok()
        .filter(|m| m.len() <= MAX_SESSION_BYTES)
        .and_then(|_| std::fs::read(path).ok())
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    saved.messages = bounded(std::mem::take(&mut saved.messages));
    if !saved.keep_history {
        saved.messages.clear();
    }
    saved
}

/// Writes the session atomically (temporary file, then rename) with
/// owner-only permissions. Without `keep_history`, no conversation text is
/// written at all.
pub fn save(path: &Path, session: &SavedSession) -> std::io::Result<()> {
    let bytes = serde_json::to_vec_pretty(session).map_err(std::io::Error::other)?;
    write_private(path, &bytes)
}

/// Creates the folder James keeps its files in, owner-only on Unix.
pub fn create_private_dir(dir: &Path) -> std::io::Result<()> {
    if dir.as_os_str().is_empty() {
        return Ok(());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)?;
    }
    #[cfg(not(unix))]
    std::fs::create_dir_all(dir)?;
    tighten(dir, true)
}

/// Removes group and world access from James's own folder or file.
fn tighten(path: &Path, dir: bool) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if path.as_os_str().is_empty() {
            return Ok(());
        }
        let meta = std::fs::symlink_metadata(path)?;
        // Never follow or change a link someone else placed here.
        if meta.file_type().is_symlink() || meta.is_dir() != dir {
            return Ok(());
        }
        if meta.permissions().mode() & 0o077 != 0 {
            let mode = if dir { 0o700 } else { 0o600 };
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))?;
        }
    }
    #[cfg(not(unix))]
    let _ = (path, dir);
    Ok(())
}

/// Writes a whole file privately and atomically, and syncs it to disk.
pub fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().unwrap_or(Path::new(""));
    create_private_dir(parent)?;
    let mut temp = path.as_os_str().to_owned();
    temp.push(".tmp");
    let temp = PathBuf::from(temp);
    // A leftover from a crash may have other permissions; start clean.
    match std::fs::remove_file(&temp) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e),
        _ => {}
    }
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temp)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    std::fs::rename(&temp, path)?;
    sync_dir(parent);
    Ok(())
}

/// Makes a rename or new file durable on Unix by syncing its folder.
pub fn sync_dir(dir: &Path) {
    #[cfg(unix)]
    if !dir.as_os_str().is_empty()
        && let Ok(handle) = std::fs::File::open(dir)
    {
        let _ = handle.sync_all();
    }
    #[cfg(not(unix))]
    let _ = dir;
}

/// Deletes the conversation, settings and crash report. An unresolved action
/// record is deliberately kept: it is removed only when the person confirms
/// they checked the target. Returns what was removed.
pub fn delete_all(session: &Path) -> Result<Vec<PathBuf>, String> {
    let mut temp = session.as_os_str().to_owned();
    temp.push(".tmp");
    let mut removed = Vec::new();
    for path in [
        session.to_path_buf(),
        PathBuf::from(temp),
        crash_report_path(session),
    ] {
        match std::fs::remove_file(&path) {
            Ok(()) => removed.push(path),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("Could not delete {}: {e}", path.display())),
        }
    }
    if let Some(parent) = session.parent() {
        sync_dir(parent);
    }
    Ok(removed)
}

#[cfg(test)]
pub(crate) fn scratch_dir(name: &str) -> PathBuf {
    // The clock alone can repeat: Windows counts in 100 ns steps, and tests
    // that start together then share a folder and collide on its files.
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "james-{name}-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_sessions_load_with_safe_defaults() {
        let saved: SavedSession =
            serde_json::from_str(r#"{"port":1234,"model":"local","messages":[]}"#).unwrap();
        assert_eq!(saved.lab_port, 5173);
        assert_eq!(saved.provider, LocalProvider::LmStudio);
        assert!(saved.keep_history);
        assert_eq!(saved.preferences, Preferences::default());
        // An upgrade with a working model skips the tour; a blank one starts it.
        assert_eq!(saved.setup().step, SetupStep::Done);
        assert_eq!(SavedSession::default().setup().step, SetupStep::Welcome);
        assert!(
            !serde_json::to_string(&saved)
                .unwrap()
                .contains("screenshot")
        );
    }

    #[test]
    fn saved_messages_are_bounded_and_well_formed() {
        let mut messages = vec![ChatMessage::new("system", "never restored")];
        messages.push(ChatMessage::new("user", "x".repeat(MAX_MESSAGE_BYTES + 1)));
        for i in 0..100 {
            messages.push(ChatMessage::new("user", format!("q{i}")));
        }
        let kept = bounded(messages);
        assert_eq!(kept.len(), MAX_SAVED_MESSAGES);
        assert_eq!(kept.last().unwrap().content, "q99");
        assert!(
            kept.iter()
                .all(|m| m.role == "user" && m.content.len() < 10)
        );
        assert!(
            Preferences {
                text_scale: f32::NAN,
                ..Preferences::default()
            }
            .text_scale()
                == 1.0
        );
        assert!(
            Preferences {
                text_scale: 9.0,
                ..Preferences::default()
            }
            .text_scale()
                == 2.0
        );
    }

    #[test]
    fn sessions_round_trip_and_history_can_be_left_unsaved() {
        let dir = scratch_dir("session");
        let path = dir.join("session.json");
        let session = SavedSession {
            provider: LocalProvider::Ollama,
            port: 11434,
            model: "qwen:latest".into(),
            messages: vec![ChatMessage::new("user", "A question")],
            preferences: Preferences {
                theme: ThemeChoice::Dark,
                high_contrast: true,
                text_scale: 1.5,
                reduce_motion: true,
                speak_replies: false,
                guide_marks: true,
            },
            setup: Some(Setup {
                step: SetupStep::Test,
                test_passed: false,
            }),
            last_version: "0.4.0".into(),
            ..SavedSession::default()
        };
        save(&path, &session).unwrap();
        let loaded = load(&path);
        assert_eq!(loaded.provider, LocalProvider::Ollama);
        assert_eq!(loaded.messages, session.messages);
        assert_eq!(loaded.preferences, session.preferences);
        assert_eq!(loaded.setup().step, SetupStep::Test);
        assert_eq!(loaded.last_version, "0.4.0");

        // Without history, a stray message in the file is not restored either.
        let unsaved = SavedSession {
            keep_history: false,
            ..session
        };
        save(&path, &unsaved).unwrap();
        assert!(load(&path).messages.is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[cfg(unix)]
    #[test]
    fn files_are_private_to_the_owner() {
        use std::os::unix::fs::PermissionsExt;
        let dir = scratch_dir("private");
        let path = dir.join("session.json");
        save(&path, &SavedSession::default()).unwrap();
        let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&dir), 0o700);
        assert_eq!(mode(&path), 0o600);
        // A file from an older version, readable by everyone, is tightened on load.
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        // An action record left by 0.3, which did not restrict permissions.
        let intent = intent_path(&path);
        std::fs::write(&intent, "{}").unwrap();
        std::fs::set_permissions(&intent, std::fs::Permissions::from_mode(0o644)).unwrap();
        load(&path);
        assert_eq!(
            (mode(&dir), mode(&path), mode(&intent)),
            (0o700, 0o600, 0o600)
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn delete_all_keeps_an_unresolved_action_record() {
        let dir = scratch_dir("delete");
        let session = dir.join("session.json");
        save(&session, &SavedSession::default()).unwrap();
        write_private(&crash_report_path(&session), b"report").unwrap();
        write_private(&intent_path(&session), b"{}").unwrap();
        let removed = delete_all(&session).unwrap();
        assert_eq!(removed.len(), 2);
        assert!(!session.exists() && !crash_report_path(&session).exists());
        assert!(intent_path(&session).exists());
        // Nothing left to delete is not an error.
        assert!(delete_all(&session).unwrap().is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }
}
