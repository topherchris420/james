//! macOS and Linux helpers built on standard system tools. Tools are resolved
//! only from fixed system directories, never through the search path. Text to
//! speak travels over stdin (or as one argument after `--`), never a shell.
use crate::{
    Action, Cancellation, Error, Listening, Screenshot, Target, VoiceConfig, png_size, supervise,
};
use std::{
    os::unix::fs::DirBuilderExt,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

const TOOL_DIRS: &[&str] = &[
    "/usr/bin",
    "/bin",
    "/usr/local/bin",
    "/opt/homebrew/bin",
    "/run/current-system/sw/bin",
];
const CAPTURE_LIMIT: Duration = Duration::from_secs(30);
const SPEECH_LIMIT: Duration = Duration::from_secs(600);

pub const USES_WHISPER: bool = true;
pub const CAN_CONTROL_INPUT: bool = false;
pub const VOICE_SUPPORT: &str = if cfg!(target_os = "macos") {
    "macOS: listening uses whisper.cpp on this computer; answers use the built-in system voice."
} else {
    "Linux: listening uses whisper.cpp on this computer; answers use espeak-ng or speech-dispatcher."
};

pub(crate) fn find_tool(name: &str) -> Option<PathBuf> {
    TOOL_DIRS
        .iter()
        .map(|dir| Path::new(dir).join(name))
        .find(|path| path.is_file())
}

/// Runs `work` with a fresh owner-only (0700) temporary folder, then deletes
/// the folder and everything in it, whatever the outcome.
pub(crate) fn with_private_dir<T>(
    work: impl FnOnce(&Path) -> Result<T, Error>,
) -> Result<T, Error> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let dir = std::env::temp_dir().join(format!(
        "james-{}-{nanos}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    // create (not create_all) fails if the path already exists, so a planted
    // folder or symlink is never reused.
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&dir)
        .map_err(|e| Error::Failed(format!("Cannot prepare a private temporary folder: {e}")))?;
    let result = work(&dir);
    let _ = std::fs::remove_dir_all(&dir);
    result
}

pub fn listening(config: &VoiceConfig) -> Listening {
    crate::whisper::listening(config)
}

pub fn whisper_models_dir() -> Option<PathBuf> {
    crate::whisper::models_dir()
}

pub fn dictate(
    timeout: Duration,
    config: &VoiceConfig,
    cancel: &Cancellation,
) -> Result<String, Error> {
    crate::whisper::dictate(timeout, config, cancel)
}

pub fn capture_screen(cancel: &Cancellation) -> Result<Screenshot, Error> {
    let png = if cfg!(target_os = "macos") {
        let tool = find_tool("screencapture")
            .ok_or_else(|| Error::Failed("macOS screencapture was not found".into()))?;
        // -x: no shutter sound, -m: main monitor only.
        via_private_file(cancel, |path| {
            let mut command = Command::new(&tool);
            command.args(["-x", "-m", "-t", "png"]).arg(path);
            command
        })?
    } else {
        capture_linux(cancel)?
    };
    let (width, height) = png_size(&png).ok_or_else(|| {
        Error::Failed("Screen capture did not return a PNG image. On macOS, allow Screen Recording for James in System Settings.".into())
    })?;
    Ok(Screenshot {
        png,
        width,
        height,
        target: None,
        james_hidden: None,
    })
}

fn capture_linux(cancel: &Cancellation) -> Result<Vec<u8>, Error> {
    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
    if wayland && let Some(grim) = find_tool("grim") {
        let mut command = Command::new(grim);
        command.args(["-t", "png", "-"]);
        if let Ok(png) = supervise(command, &[], CAPTURE_LIMIT, cancel) {
            return Ok(png);
        }
        // grim needs wlroots screencopy; GNOME and KDE fall through below.
    }
    if let Some(tool) = find_tool("gnome-screenshot") {
        return via_private_file(cancel, |path| {
            let mut command = Command::new(&tool);
            command.arg("-f").arg(path);
            command
        });
    }
    if let Some(tool) = find_tool("spectacle") {
        return via_private_file(cancel, |path| {
            let mut command = Command::new(&tool);
            command.args(["-b", "-n", "-f", "-o"]).arg(path);
            command
        });
    }
    if !wayland && let Some(import) = find_tool("import") {
        let mut command = Command::new(import);
        command.args(["-window", "root", "png:-"]);
        return supervise(command, &[], CAPTURE_LIMIT, cancel);
    }
    Err(Error::Failed(
        "No screen capture tool found. Install gnome-screenshot, spectacle, grim (wlroots Wayland) or ImageMagick (X11).".into(),
    ))
}

/// For tools that can only write a file: capture into a private folder, read
/// the image into memory, and delete it immediately.
fn via_private_file(
    cancel: &Cancellation,
    command: impl FnOnce(&Path) -> Command,
) -> Result<Vec<u8>, Error> {
    with_private_dir(|dir| {
        let path = dir.join("screen.png");
        supervise(command(&path), &[], CAPTURE_LIMIT, cancel)?;
        let size = std::fs::metadata(&path)
            .map_err(|_| Error::Failed("Screen capture produced no image (permission denied or cancelled in the OS dialog)".into()))?
            .len();
        if size > 64 * 1024 * 1024 {
            return Err(Error::Failed("Screen capture exceeded 64 MiB".into()));
        }
        std::fs::read(&path).map_err(|e| Error::Failed(e.to_string()))
    })
}

pub fn speak(text: &str, cancel: &Cancellation) -> Result<(), Error> {
    if cfg!(target_os = "macos") {
        let say = find_tool("say")
            .ok_or_else(|| Error::Failed("macOS speech (say) was not found".into()))?;
        let mut command = Command::new(say);
        command.args(["-f", "-"]);
        return supervise(command, text.as_bytes(), SPEECH_LIMIT, cancel).map(|_| ());
    }
    if let Some(espeak) = find_tool("espeak-ng").or_else(|| find_tool("espeak")) {
        let mut command = Command::new(espeak);
        command.arg("--stdin");
        return supervise(command, text.as_bytes(), SPEECH_LIMIT, cancel).map(|_| ());
    }
    if let Some(spd) = find_tool("spd-say") {
        let mut command = Command::new(spd);
        command.args(["--wait", "--", text]);
        return supervise(command, &[], SPEECH_LIMIT, cancel).map(|_| ());
    }
    Err(Error::Failed(
        "No speech voice found. Install espeak-ng (for example: sudo apt install espeak-ng) to hear James.".into(),
    ))
}

pub fn execute(
    _: &Action,
    _: u32,
    _: u32,
    _: Option<&Target>,
    _: &Cancellation,
) -> Result<(), Error> {
    Err(Error::Unsupported(
        "Reviewed clicks and typing are available on Windows only for now.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tools_resolve_only_from_fixed_directories() {
        assert!(find_tool("definitely-not-a-james-tool").is_none());
        if let Some(path) = find_tool("sh") {
            assert!(path.is_absolute());
        }
    }
    #[test]
    fn private_dirs_are_owner_only_and_always_removed() {
        use std::os::unix::fs::PermissionsExt;
        let mut seen = None;
        let result: Result<(), Error> = with_private_dir(|dir| {
            let mode = std::fs::metadata(dir).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o700);
            std::fs::write(dir.join("secret"), b"x").unwrap();
            seen = Some(dir.to_path_buf());
            Err(Error::Failed("boom".into()))
        });
        assert!(result.is_err());
        assert!(!seen.unwrap().exists());
    }
    #[test]
    fn input_is_unsupported_and_says_so() {
        assert!(matches!(
            execute(
                &Action::Click { x: 1, y: 1 },
                10,
                10,
                None,
                &Cancellation::default()
            ),
            Err(Error::Unsupported(_))
        ));
    }
}
