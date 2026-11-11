//! Windows now-playing information and explicitly requested media controls.
//! Query only while the widget is visible; operations block and belong on a worker.

use serde::Deserialize;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct NowPlaying {
    pub title: String,
    pub artist: String,
    pub source: String,
    pub playing: bool,
    pub can_toggle: bool,
    pub can_next: bool,
    // Retain the exact identifier for the control check; UI shows `source`.
    source_id: String,
    checked_at: Instant,
}

#[derive(Clone, Copy, Debug)]
pub enum MediaAction {
    Toggle,
    Next,
}

#[derive(Deserialize)]
struct Snapshot {
    active: bool,
    #[serde(default)]
    title: String,
    #[serde(default)]
    artist: String,
    #[serde(default)]
    source: String,
    #[serde(default)]
    playing: bool,
    #[serde(default)]
    can_toggle: bool,
    #[serde(default)]
    can_next: bool,
}

fn clean(value: &str) -> String {
    value
        .chars()
        .filter(|c| !james_platform::is_unreviewable(*c))
        .take(256)
        .collect()
}

fn parse_snapshot(data: &[u8]) -> Result<Option<NowPlaying>, String> {
    let snapshot: Snapshot = serde_json::from_slice(data)
        .map_err(|_| "Windows returned invalid media information.".to_owned())?;
    if !snapshot.active {
        return Ok(None);
    }
    if snapshot.source.is_empty() || snapshot.source.len() > 2048 {
        return Err("The media source could not be identified.".into());
    }
    Ok(Some(NowPlaying {
        title: clean(&snapshot.title),
        artist: clean(&snapshot.artist),
        source: clean(&snapshot.source),
        playing: snapshot.playing,
        can_toggle: snapshot.can_toggle,
        can_next: snapshot.can_next,
        source_id: snapshot.source,
        checked_at: Instant::now(),
    }))
}

/// Reads the current OS media session. No playback command is issued.
pub fn query(cancel: &james_platform::Cancellation) -> Result<Option<NowPlaying>, String> {
    let response = run_helper(&serde_json::json!({"action": "query"}), cancel)?;
    parse_snapshot(&response)
}

/// Requires a recent displayed snapshot and an explicit click. Windows checks
/// that its current session still belongs to the same source app. Multiple
/// sessions from one app cannot be distinguished by that source identifier.
pub fn control(
    snapshot: &NowPlaying,
    action: MediaAction,
    cancel: &james_platform::Cancellation,
) -> Result<(), String> {
    if snapshot.checked_at.elapsed() > Duration::from_secs(30) {
        return Err(
            "Media information is out of date. Refresh before controlling playback.".into(),
        );
    }
    let action = match action {
        MediaAction::Toggle if snapshot.can_toggle => "toggle",
        MediaAction::Next if snapshot.can_next => "next",
        _ => return Err("This media player does not currently support that control.".into()),
    };
    let response = run_helper(
        &serde_json::json!({"action": action, "source": snapshot.source_id}),
        cancel,
    )?;
    let result: serde_json::Value = serde_json::from_slice(&response)
        .map_err(|_| "Windows returned an invalid media-control response.".to_owned())?;
    if result.get("ok").and_then(serde_json::Value::as_bool) != Some(true) {
        return Err("The media player did not accept the request.".into());
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn run_helper(_: &serde_json::Value, _: &james_platform::Cancellation) -> Result<Vec<u8>, String> {
    Err("Now-playing information is currently supported on Windows only.".into())
}

#[cfg(target_os = "windows")]
fn run_helper(
    request: &serde_json::Value,
    cancel: &james_platform::Cancellation,
) -> Result<Vec<u8>, String> {
    use std::io::{Read, Write};
    use std::os::windows::process::CommandExt;
    if cancel.is_cancelled() {
        return Err("Media request cancelled.".into());
    }
    let root = std::env::var_os("SystemRoot")
        .ok_or_else(|| "Windows system directory is unavailable.".to_owned())?;
    let executable = std::path::PathBuf::from(root)
        .join("System32")
        .join("WindowsPowerShell")
        .join("v1.0")
        .join("powershell.exe");
    let mut child = std::process::Command::new(executable)
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            include_str!("media_now.ps1"),
        ])
        .creation_flags(0x0800_0000)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|error| format!("Could not start media helper: {error}"))?;
    let input =
        serde_json::to_vec(request).map_err(|_| "Could not prepare media request.".to_owned())?;
    let mut stdin = child.stdin.take().expect("piped stdin");
    let writer = std::thread::spawn(move || stdin.write_all(&input));
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let out = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.take(16385).read_to_end(&mut bytes).map(|_| bytes)
    });
    let err = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.take(4097).read_to_end(&mut bytes).map(|_| bytes)
    });
    let start = Instant::now();
    let status = loop {
        if cancel.is_cancelled() || start.elapsed() >= Duration::from_secs(15) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = writer.join();
            let _ = out.join();
            let _ = err.join();
            return Err(if cancel.is_cancelled() {
                "Media request cancelled.".into()
            } else {
                "Media request timed out.".into()
            });
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => std::thread::sleep(Duration::from_millis(40)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = writer.join();
                let _ = out.join();
                let _ = err.join();
                return Err(format!("Media helper failed: {error}"));
            }
        }
    };
    writer
        .join()
        .map_err(|_| "Media request writer failed.".to_owned())?
        .map_err(|_| "Could not send media request.".to_owned())?;
    let output = out
        .join()
        .map_err(|_| "Media output reader failed.".to_owned())?
        .map_err(|_| "Could not read media output.".to_owned())?;
    let errors = err
        .join()
        .map_err(|_| "Media error reader failed.".to_owned())?
        .map_err(|_| "Could not read media error.".to_owned())?;
    if output.len() > 16384 || errors.len() > 4096 {
        return Err("Media helper output exceeded its limit.".into());
    }
    if !status.success() {
        let detail = clean(&String::from_utf8_lossy(&errors));
        return Err(if detail.is_empty() {
            "Windows media information is unavailable.".into()
        } else {
            detail
        });
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_and_sanitizes_session_without_touching_media() {
        assert!(parse_snapshot(br#"{"active":false}"#).unwrap().is_none());
        let item = parse_snapshot("{\"active\":true,\"title\":\"Song\\u202e\",\"artist\":\"Singer\",\"source\":\"player\",\"playing\":true,\"can_toggle\":true,\"can_next\":false}".as_bytes()).unwrap().unwrap();
        assert_eq!(item.title, "Song");
        assert!(item.playing && item.can_toggle && !item.can_next);
        assert!(parse_snapshot(br#"{"active":true,"source":""}"#).is_err());
    }
    #[test]
    fn stale_and_unsupported_controls_fail_before_touching_media() {
        let mut item = parse_snapshot(
            br#"{"active":true,"source":"player","can_toggle":true,"can_next":false}"#,
        )
        .unwrap()
        .unwrap();
        let cancel = james_platform::Cancellation::default();
        assert!(control(&item, MediaAction::Next, &cancel).is_err());
        item.checked_at = Instant::now() - Duration::from_secs(31);
        assert!(control(&item, MediaAction::Toggle, &cancel).is_err());
    }
}
