//! Inspect a foreground window and deliver explicitly reviewed dictation on
//! Windows without using the clipboard. A focused native window does not prove
//! which field inside a browser or custom control receives keyboard input.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct DictationTarget {
    pub window: i64,
    pub focused_window: i64,
    pub process_id: u32,
    pub focused_process_id: u32,
    pub process_name: String,
    pub title: String,
    pub focus_class: String,
}

impl DictationTarget {
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    fn validate(mut self, own_process: u32) -> Result<Self, String> {
        if self.window == 0
            || self.focused_window == 0
            || self.process_id == 0
            || self.focused_process_id == 0
            || self.process_id == own_process
            || self.focused_process_id == own_process
        {
            return Err(
                "No other application was in front. Switch to the destination app and try again."
                    .into(),
            );
        }
        // Window titles and process names are owned by other applications. Do
        // not let them inject controls or an unbounded label into the preview.
        fn clean(value: &str) -> String {
            value
                .chars()
                .filter(|c| !james_platform::is_unreviewable(*c))
                .take(256)
                .collect()
        }
        self.process_name = clean(&self.process_name);
        self.title = clean(&self.title);
        self.focus_class = clean(&self.focus_class);
        Ok(self)
    }
}

/// Inspect, but never activate, the foreground app. Call on a worker thread
/// immediately after the shortcut and display the result for review. Since an
/// app may change while the helper starts, the name is advisory; it cannot
/// establish a field identity or promise a destination for keyboard input.
#[cfg(target_os = "windows")]
pub fn inspect_foreground(
    cancel: &james_platform::Cancellation,
) -> Result<DictationTarget, String> {
    let output = run_helper(include_str!("dictation_target.ps1"), b"{}", cancel)?;
    let target: DictationTarget = serde_json::from_slice(&output)
        .map_err(|_| "Windows returned an invalid destination window.".to_owned())?;
    target.validate(std::process::id())
}

/// Delivers exactly the reviewed text after checking that the same window,
/// process and native focused control are still active. This uses SendInput
/// with Unicode events and does not read or modify any clipboard formats.
/// Windows cannot prove which internal field a custom control routes input to,
/// and another app may change focus immediately after the check. A successful
/// call only means Windows accepted the events, not that the app inserted them.
/// A failed call can mean partial insertion: never retry it automatically.
#[cfg(target_os = "windows")]
pub fn deliver_reviewed(
    target: &DictationTarget,
    text: &str,
    cancel: &james_platform::Cancellation,
) -> Result<(), String> {
    validate_delivery(target, text, std::process::id())?;
    let request =
        serde_json::json!({"target": target, "text": text, "own_process": std::process::id()});
    let input =
        serde_json::to_vec(&request).map_err(|_| "Could not prepare reviewed text.".to_owned())?;
    run_helper(include_str!("dictation_deliver.ps1"), &input, cancel)
        .map(|_| ())
        .map_err(|error| {
            format!(
                "{error} Inspect the destination before retrying: some text may have been inserted."
            )
        })
}

#[cfg(not(target_os = "windows"))]
pub fn deliver_reviewed(
    _: &DictationTarget,
    _: &str,
    _: &james_platform::Cancellation,
) -> Result<(), String> {
    Err("Dictation delivery is currently available on Windows only.".into())
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn validate_delivery(target: &DictationTarget, text: &str, own_process: u32) -> Result<(), String> {
    target.clone().validate(own_process)?;
    let units = text.encode_utf16().count();
    if units == 0 || units > 2000 || text.chars().any(james_platform::is_unreviewable) {
        return Err("Reviewed text must be 1–2000 UTF-16 units with no control or hidden formatting characters.".into());
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn run_helper(
    script: &str,
    input: &[u8],
    cancel: &james_platform::Cancellation,
) -> Result<Vec<u8>, String> {
    use std::io::{Read, Write};
    use std::os::windows::process::CommandExt;
    use std::time::{Duration, Instant};
    if cancel.is_cancelled() {
        return Err("Operation cancelled.".into());
    }
    let root = std::env::var_os("SystemRoot")
        .ok_or_else(|| "Windows could not locate its system directory.".to_owned())?;
    let powershell = std::path::PathBuf::from(root)
        .join("System32")
        .join("WindowsPowerShell")
        .join("v1.0")
        .join("powershell.exe");
    let mut child = std::process::Command::new(powershell)
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            script,
        ])
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|error| format!("Could not inspect the destination window: {error}"))?;
    let mut stdin = child.stdin.take().expect("piped stdin");
    let input = input.to_vec();
    let writer = std::thread::spawn(move || stdin.write_all(&input));
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let out = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.take(8193).read_to_end(&mut bytes).map(|_| bytes)
    });
    let err = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.take(8193).read_to_end(&mut bytes).map(|_| bytes)
    });
    let started = Instant::now();
    let status = loop {
        if cancel.is_cancelled() || started.elapsed() >= Duration::from_secs(20) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = writer.join();
            let _ = out.join();
            let _ = err.join();
            return Err(if cancel.is_cancelled() {
                "Operation cancelled.".into()
            } else {
                "Windows helper timed out.".into()
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
                return Err(format!("Windows helper failed: {error}"));
            }
        }
    };
    writer
        .join()
        .map_err(|_| "Request writer failed.".to_owned())?
        .map_err(|_| "Could not send the reviewed request to Windows.".to_owned())?;
    let output = out
        .join()
        .map_err(|_| "Output reader failed.".to_owned())?
        .map_err(|_| "Could not read helper output.".to_owned())?;
    let errors = err
        .join()
        .map_err(|_| "Error reader failed.".to_owned())?
        .map_err(|_| "Could not read helper errors.".to_owned())?;
    if output.len() > 8192 || errors.len() > 8192 {
        return Err("Windows helper output exceeded its limit.".into());
    }
    if !status.success() {
        let message: String = String::from_utf8_lossy(&errors)
            .chars()
            .filter(|c| !james_platform::is_unreviewable(*c))
            .take(500)
            .collect();
        return Err(if message.is_empty() {
            "Windows could not complete the request. Check the destination before retrying.".into()
        } else {
            message
        });
    }
    Ok(output)
}

#[cfg(not(target_os = "windows"))]
pub fn inspect_foreground(_: &james_platform::Cancellation) -> Result<DictationTarget, String> {
    Err("Destination inspection is currently available on Windows only.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target() -> DictationTarget {
        DictationTarget {
            window: 123,
            focused_window: 234,
            process_id: 12,
            focused_process_id: 12,
            process_name: "editor".into(),
            title: "Document".into(),
            focus_class: "Edit".into(),
        }
    }

    #[test]
    fn rejects_missing_or_own_foreground() {
        assert!(target().validate(12).is_err());
        let mut missing = target();
        missing.window = 0;
        assert!(missing.validate(13).is_err());
        let mut missing_process = target();
        missing_process.process_id = 0;
        assert!(missing_process.validate(13).is_err());
    }

    #[test]
    fn bounds_untrusted_preview_labels() {
        let mut value = target();
        value.title = format!("A\n\u{202e}{}", "x".repeat(500));
        let checked = value.validate(13).unwrap();
        assert_eq!(checked.title.chars().count(), 256);
        assert!(!checked.title.contains('\n'));
        assert!(!checked.title.contains('\u{202e}'));
    }

    #[test]
    fn reviewed_text_is_bounded_and_single_line() {
        assert!(validate_delivery(&target(), "Hi 😀", 13).is_ok());
        assert!(validate_delivery(&target(), "", 13).is_err());
        assert!(validate_delivery(&target(), "bad\ntext", 13).is_err());
        assert!(validate_delivery(&target(), "bad\u{202e}text", 13).is_err());
        assert!(validate_delivery(&target(), "bad\u{200b}text", 13).is_err());
        assert!(validate_delivery(&target(), &"x".repeat(2001), 13).is_err());
        assert!(validate_delivery(&target(), "hello", 12).is_err());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn helper_cancel_and_output_limit_do_not_touch_any_apps() {
        let cancelled = james_platform::Cancellation::default();
        cancelled.cancel();
        assert!(run_helper("Start-Sleep -Seconds 30", b"{}", &cancelled).is_err());
        let cancel = james_platform::Cancellation::default();
        let stopping = cancel.clone();
        let thread =
            std::thread::spawn(move || run_helper("Start-Sleep -Seconds 30", b"{}", &stopping));
        std::thread::sleep(std::time::Duration::from_millis(300));
        cancel.cancel();
        assert!(thread.join().unwrap().is_err());
        assert!(
            run_helper(
                "[Console]::Write(('x' * 9000))",
                b"{}",
                &james_platform::Cancellation::default()
            )
            .is_err()
        );
    }
}
