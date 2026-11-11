//! Open one reviewed, allowlisted Windows Settings page. This changes no setting.
//!
//! URI identities were checked against Microsoft's primary documentation:
//! https://learn.microsoft.com/windows/uwp/launch-resume/launch-settings-app
//! Page availability depends on the Windows version and policy. A successful
//! dispatch does not prove that a page became visible. Once dispatch starts,
//! cancellation cannot promise that Windows did not already open the page.

use james_models::ChatMessage;
use james_platform::Cancellation;
use serde::{Deserialize, Serialize};

const MAX_INSTRUCTION_BYTES: usize = 4096;
const MAX_RESPONSE_BYTES: usize = 4096;
const SYSTEM_PROMPT: &str = "Choose exactly one Windows Settings page to help with the user's request. Allowed page names: display, sound, microphone_privacy, bluetooth, keyboard_accessibility, notifications. Reply with exactly one JSON object containing page (one allowed name) and reason (a short single-line explanation). Do not include a URI, command, executable, extra fields, Markdown, or instructions to execute. Opening a page does not change settings. The person must review the selected page before it can open.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingsPage {
    Display,
    Sound,
    MicrophonePrivacy,
    Bluetooth,
    KeyboardAccessibility,
    Notifications,
}

impl SettingsPage {
    pub const ALL: [Self; 6] = [
        Self::Display,
        Self::Sound,
        Self::MicrophonePrivacy,
        Self::Bluetooth,
        Self::KeyboardAccessibility,
        Self::Notifications,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Display => "Display",
            Self::Sound => "Sound",
            Self::MicrophonePrivacy => "Microphone privacy",
            Self::Bluetooth => "Bluetooth",
            Self::KeyboardAccessibility => "Keyboard accessibility",
            Self::Notifications => "Notifications",
        }
    }

    /// Fixed application constants, never model-provided URLs or arguments.
    pub fn uri(self) -> &'static str {
        match self {
            Self::Display => "ms-settings:display",
            Self::Sound => "ms-settings:sound",
            Self::MicrophonePrivacy => "ms-settings:privacy-microphone",
            Self::Bluetooth => "ms-settings:bluetooth",
            Self::KeyboardAccessibility => "ms-settings:easeofaccess-keyboard",
            Self::Notifications => "ms-settings:notifications",
        }
    }
}

/// Immutable proposed destination; parsing a proposal never opens anything.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Proposal {
    page: SettingsPage,
    reason: String,
}

impl Proposal {
    pub fn page(&self) -> SettingsPage {
        self.page
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// A manually chosen destination still goes through the same review UI.
    pub fn manual(page: SettingsPage) -> Self {
        Self {
            page,
            reason: "Settings page selected by you.".into(),
        }
    }
}

/// Isolated request: no history, screenshots, workspace text or attachments.
pub fn messages(instruction: &str) -> Result<Vec<ChatMessage>, String> {
    if instruction.trim().is_empty()
        || instruction.len() > MAX_INSTRUCTION_BYTES
        || instruction.contains('\0')
    {
        return Err(
            "Enter a settings request between 1 and 4096 bytes without NUL characters.".into(),
        );
    }
    Ok(vec![
        ChatMessage::new("system", SYSTEM_PROMPT),
        ChatMessage::new("user", instruction),
    ])
}

/// Accept exactly the small documented response. URLs, commands, unknown page
/// names, extra fields, code fences and unreviewable explanations are refused.
pub fn parse_proposal(reply: &str) -> Result<Proposal, String> {
    if reply.len() > MAX_RESPONSE_BYTES {
        return Err("Settings proposal exceeded 4096 bytes.".into());
    }
    let proposal: Proposal = serde_json::from_str(reply)
        .map_err(|_| "Expected one supported settings page and a reason in JSON.".to_owned())?;
    if proposal.reason.trim().is_empty()
        || proposal.reason.len() > 512
        || proposal.reason.chars().any(james_platform::is_unreviewable)
    {
        return Err("The settings explanation must be 1–512 bytes on one visible line.".into());
    }
    Ok(proposal)
}

/// One-use permission for an exact reviewed destination. Construct only after
/// the person clicks the specific Open button showing this proposal's page.
#[derive(Debug)]
pub struct ReviewedOpen {
    page: SettingsPage,
}

impl ReviewedOpen {
    pub fn approve(proposal: Proposal) -> Self {
        Self {
            page: proposal.page,
        }
    }
}

/// Consume a reviewed destination once. Do not retry on cancellation or helper
/// failure: the Settings page might already have opened. Run on a worker thread.
pub fn open_reviewed(review: ReviewedOpen, cancel: &Cancellation) -> Result<(), String> {
    open_with(review, cancel, run_helper)
}

fn open_with(
    review: ReviewedOpen,
    cancel: &Cancellation,
    dispatch: impl FnOnce(SettingsPage, &Cancellation) -> Result<(), String>,
) -> Result<(), String> {
    if cancel.is_cancelled() {
        return Err("Settings request cancelled before dispatch.".into());
    }
    dispatch(review.page, cancel)
}

#[cfg(not(target_os = "windows"))]
fn run_helper(_: SettingsPage, _: &Cancellation) -> Result<(), String> {
    Err("Opening settings pages is currently supported on Windows only.".into())
}

#[cfg(target_os = "windows")]
fn run_helper(page: SettingsPage, cancel: &Cancellation) -> Result<(), String> {
    use std::io::{Read, Write};
    use std::os::windows::process::CommandExt;
    use std::time::{Duration, Instant};

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Response {
        opened: bool,
        page: SettingsPage,
    }

    let input = serde_json::to_vec(&serde_json::json!({"page": page}))
        .map_err(|_| "Could not prepare the reviewed settings request.".to_owned())?;
    let root = std::env::var_os("SystemRoot")
        .ok_or_else(|| "Windows system directory is unavailable.".to_owned())?;
    let executable = std::path::PathBuf::from(root)
        .join("System32")
        .join("WindowsPowerShell")
        .join("v1.0")
        .join("powershell.exe");
    if cancel.is_cancelled() {
        return Err("Settings request cancelled before dispatch.".into());
    }
    let mut child = std::process::Command::new(executable)
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            include_str!("settings_page.ps1"),
        ])
        .creation_flags(0x0800_0000)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|error| format!("Could not start the settings helper: {error}"))?;
    let mut stdin = child.stdin.take().expect("piped stdin");
    let writer = std::thread::spawn(move || stdin.write_all(&input));
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let out = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.take(1025).read_to_end(&mut bytes).map(|_| bytes)
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
                "Settings request stopped. Windows may already have opened the page.".into()
            } else {
                "Settings request timed out. Windows may already have opened the page.".into()
            });
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => std::thread::sleep(Duration::from_millis(40)),
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = writer.join();
                let _ = out.join();
                let _ = err.join();
                return Err(
                    "Settings helper failed. Windows may already have opened the page.".into(),
                );
            }
        }
    };
    // Join every pipe worker even if one failed. No helper output is displayed
    // as trusted UI text, and output is bounded independently on each stream.
    let sent = writer.join();
    let output = out.join();
    let errors = err.join();
    let valid = sent.is_ok_and(|result| result.is_ok());
    let output = output.ok().and_then(Result::ok);
    let errors = errors.ok().and_then(Result::ok);
    if !valid
        || !status.success()
        || output.as_ref().is_none_or(|bytes| bytes.len() > 1024)
        || errors.as_ref().is_none_or(|bytes| bytes.len() > 4096)
    {
        return Err(
            "Windows did not confirm the settings request. The page may already have opened."
                .into(),
        );
    }
    let result: Response =
        serde_json::from_slice(&output.expect("checked output")).map_err(|_| {
            "Windows returned an invalid settings response. The page may already have opened."
                .to_owned()
        })?;
    if !result.opened || result.page != page {
        return Err("Windows did not confirm the reviewed settings destination.".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn all_pages_have_fixed_documented_destinations() {
        let expected = [
            ("display", "Display", "ms-settings:display"),
            ("sound", "Sound", "ms-settings:sound"),
            (
                "microphone_privacy",
                "Microphone privacy",
                "ms-settings:privacy-microphone",
            ),
            ("bluetooth", "Bluetooth", "ms-settings:bluetooth"),
            (
                "keyboard_accessibility",
                "Keyboard accessibility",
                "ms-settings:easeofaccess-keyboard",
            ),
            (
                "notifications",
                "Notifications",
                "ms-settings:notifications",
            ),
        ];
        for (page, (name, label, uri)) in SettingsPage::ALL.into_iter().zip(expected) {
            assert_eq!(page.label(), label);
            assert_eq!(page.uri(), uri);
            assert_eq!(serde_json::to_value(page).unwrap(), name);
            let parsed =
                parse_proposal(&format!(r#"{{"page":"{name}","reason":"Requested page"}}"#))
                    .unwrap();
            assert_eq!(parsed.page(), page);
            assert_eq!(parsed.reason(), "Requested page");
        }
    }

    #[test]
    fn proposals_reject_urls_commands_extra_fields_and_hidden_reasons() {
        for reply in [
            r#"{"page":"https://example.org","reason":"Go"}"#,
            r#"{"page":"ms-settings:sound","reason":"Go"}"#,
            r#"{"page":"cmd /c calc","reason":"Go"}"#,
            r#"{"page":"sound","reason":"Go","uri":"ms-settings:recovery"}"#,
            r#"{"page":"sound","reason":"Go","command":"calc"}"#,
            r#"{"page":"sound","reason":""}"#,
            r#"{"page":"sound","reason":"line\nother"}"#,
            r#"{"page":"sound","reason":"hidden\u202e"}"#,
            "```json\n{\"page\":\"sound\",\"reason\":\"Go\"}\n```",
        ] {
            assert!(parse_proposal(reply).is_err(), "{reply}");
        }
        let huge_reason = serde_json::json!({"page":"sound", "reason":"x".repeat(513)});
        assert!(parse_proposal(&huge_reason.to_string()).is_err());
        assert!(parse_proposal(&"x".repeat(MAX_RESPONSE_BYTES + 1)).is_err());
    }

    #[test]
    fn request_context_is_isolated_and_instruction_is_bounded() {
        let context = messages("Help me fix the microphone").unwrap();
        assert_eq!(context.len(), 2);
        assert_eq!(context[0].role, "system");
        assert_eq!(context[1].role, "user");
        assert_eq!(context[1].content, "Help me fix the microphone");
        for text in ["", "  ", "bad\0request"] {
            assert!(messages(text).is_err());
        }
        assert!(messages(&"x".repeat(MAX_INSTRUCTION_BYTES + 1)).is_err());
    }

    #[test]
    fn cancelled_review_never_reaches_dispatch_and_exact_page_is_consumed() {
        let cancel = Cancellation::default();
        cancel.cancel();
        let called = Cell::new(false);
        let result = open_with(
            ReviewedOpen::approve(Proposal::manual(SettingsPage::Sound)),
            &cancel,
            |_, _| {
                called.set(true);
                Ok(())
            },
        );
        assert!(result.is_err());
        assert!(!called.get());
        open_with(
            ReviewedOpen::approve(Proposal::manual(SettingsPage::Bluetooth)),
            &Cancellation::default(),
            |page, _| {
                assert_eq!(page, SettingsPage::Bluetooth);
                Ok(())
            },
        )
        .unwrap();
    }
}
