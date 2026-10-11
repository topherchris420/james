//! Windows helpers: embedded PowerShell scripts launched from the absolute
//! SystemRoot path, with all variable input passed as JSON on stdin.
use crate::{Action, Cancellation, Error, Listening, Screenshot, VoiceConfig, supervise};
use base64::Engine;
use serde::Deserialize;
use std::{os::windows::process::CommandExt, path::PathBuf, process::Command, time::Duration};

pub const USES_WHISPER: bool = false;
pub const CAN_CONTROL_INPUT: bool = true;
pub const VOICE_SUPPORT: &str = "Windows: listening uses the installed Windows speech recognizer and answers use the installed Windows voice.";

pub fn listening(_: &VoiceConfig) -> Listening {
    Listening::Ready("Windows speech recognition".into())
}

pub fn whisper_models_dir() -> Option<PathBuf> {
    None
}

pub fn capture_screen(cancel: &Cancellation) -> Result<Screenshot, Error> {
    #[derive(Deserialize)]
    struct Response {
        png: String,
        width: u32,
        height: u32,
    }
    let value = run(
        include_str!("capture.ps1"),
        &serde_json::json!({}),
        // Cold Windows PowerShell/Add-Type startup can exceed 20 seconds while
        // the host is busy. Cancellation remains responsive during this bound.
        Duration::from_secs(60),
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

pub fn dictate(timeout: Duration, _: &VoiceConfig, cancel: &Cancellation) -> Result<String, Error> {
    let value = run(
        include_str!("dictate.ps1"),
        &serde_json::json!({"seconds": timeout.as_secs_f64()}),
        timeout + Duration::from_secs(10),
        cancel,
    )?;
    serde_json::from_slice(&value).map_err(|e| Error::Failed(e.to_string()))
}

pub fn speak(text: &str, cancel: &Cancellation) -> Result<(), Error> {
    run(
        include_str!("speak.ps1"),
        &serde_json::json!({"text": text}),
        Duration::from_secs(600),
        cancel,
    )
    .map(|_| ())
}

pub fn execute(
    action: &Action,
    width: u32,
    height: u32,
    cancel: &Cancellation,
) -> Result<(), Error> {
    run(
        include_str!("input.ps1"),
        &serde_json::json!({"action": action, "width": width, "height": height}),
        Duration::from_secs(15),
        cancel,
    )
    .map(|_| ())
}

fn run(
    script: &str,
    input: &serde_json::Value,
    timeout: Duration,
    cancel: &Cancellation,
) -> Result<Vec<u8>, Error> {
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
    let mut command = Command::new(powershell);
    command
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-EncodedCommand",
            &encoded,
        ])
        .creation_flags(0x08000000);
    let bytes = serde_json::to_vec(input).map_err(|e| Error::Failed(e.to_string()))?;
    supervise(command, &bytes, timeout, cancel)
}
