//! Review parsing and one-action supervision. An intent marker survives until a
//! human checks the target; no action is replayed on restart.
use james_core::{Approval, Effect, Invocation, Policy, supervisor::RunSupervisor};
use james_platform::{Action, Cancellation, ReviewedAction};
use serde::Deserialize;
use std::{collections::BTreeMap, fs::OpenOptions, io::Write, path::Path, time::Instant};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Proposal {
    pub rationale: String,
    pub action: Action,
}
pub fn parse_proposal(reply: &str) -> Result<Proposal, String> {
    if reply.len() > 16_384 {
        return Err("Proposal exceeded 16 KiB".into());
    }
    let trimmed = reply.trim();
    let json = trimmed
        .strip_prefix("```json")
        .and_then(|s| s.strip_suffix("```"))
        .or_else(|| {
            trimmed
                .strip_prefix("```")
                .and_then(|s| s.strip_suffix("```"))
        })
        .unwrap_or(trimmed);
    let proposal: Proposal = serde_json::from_str(json.trim())
        .map_err(|_| "Expected one supported JSON action".to_owned())?;
    if proposal.rationale.trim().is_empty() || proposal.rationale.len() > 2048 {
        return Err("Missing or oversized explanation".into());
    }
    Ok(proposal)
}
pub fn needs_reconciliation(path: &Path) -> bool {
    // Permission or other metadata failures are uncertain, never permission to act.
    match std::fs::metadata(path) {
        Ok(_) => true,
        Err(e) => e.kind() != std::io::ErrorKind::NotFound,
    }
}
pub fn acknowledge(path: &Path) -> Result<(), String> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("Cannot clear action record: {e}")),
    }
}
pub fn execute(review: ReviewedAction, path: &Path, cancel: &Cancellation) -> Result<(), String> {
    execute_with(review, path, cancel, james_platform::execute)
}
fn execute_with(
    review: ReviewedAction,
    path: &Path,
    cancel: &Cancellation,
    effect: impl FnOnce(ReviewedAction, &Cancellation) -> Result<(), james_platform::Error>,
) -> Result<(), String> {
    if cancel.is_cancelled() {
        return Err("Action cancelled before dispatch".into());
    }
    let start = Instant::now();
    let policy = Policy {
        revision: 1,
        allow_workspace_reads: false,
        tools_enabled: true,
    };
    let exact = serde_json::to_string(review.action()).map_err(|e| e.to_string())?;
    let invocation = Invocation::new(
        1,
        "desktop.input",
        "james-platform:v1",
        BTreeMap::from([("action".into(), exact.clone())]),
        "primary-monitor",
        "foreground-application",
        Effect::DesktopControl,
    )
    .map_err(|e| e.to_string())?;
    let mut run = RunSupervisor::new(1, 1, 0, 20_000).map_err(|e| e.to_string())?;
    let id = run.propose(invocation).map_err(|e| e.to_string())?;
    run.prepare(id, policy).map_err(|e| e.to_string())?;
    // The non-cloneable ReviewedAction can only come from the Allow once UI.
    let approval =
        Approval::after_user_review(run.action(id).expect("proposed action"), policy, 0, 20_000)
            .map_err(|e| e.to_string())?;
    run.approve(id, approval, 0).map_err(|e| e.to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    run.dispatch_with_journal(id, policy, start.elapsed().as_millis() as u64, |_, _| {
        if cancel.is_cancelled() { return Err(()); }
        // create_new is also a cross-process gate. Never overwrite an unresolved intent.
        let mut file = OpenOptions::new().write(true).create_new(true).open(path).map_err(|_| ())?;
        let record = serde_json::json!({"schema":"james-action-intent/v1", "action":serde_json::from_str::<serde_json::Value>(&exact).map_err(|_| ())?, "state":"requires-human-verification"});
        file.write_all(record.to_string().as_bytes()).map_err(|_| ())?;
        file.sync_all().map_err(|_| ())
    }).map_err(|e| format!("Input blocked: {e}. Inspect any previous action record before continuing."))?;
    if cancel.is_cancelled() {
        return Err(
            "Cancelled before input. Check the target and acknowledge the action record.".into(),
        );
    }
    match effect(review, cancel) {
        Ok(()) => {
            run.complete(id, true).map_err(|e| e.to_string())?;
            Ok(())
        }
        Err(e) => Err(format!(
            "Input outcome may be uncertain: {e}. Check the target; James will not retry."
        )),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn path() -> std::path::PathBuf {
        std::env::temp_dir()
            .join(format!(
                "james-review-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ))
            .join("intent.json")
    }
    #[test]
    fn parses_only_one_allowlisted_action() {
        assert!(
            parse_proposal(
                r#"{"rationale":"Focus field","action":{"kind":"click","x":10,"y":20}}"#
            )
            .is_ok()
        );
        for s in [
            r#"{"rationale":"Run","action":{"kind":"shell","command":"echo bad"}}"#,
            r#"{"rationale":"Go","action":{"kind":"key","key":"tab"},"execute":true}"#,
            r#"{"rationale":"Go","action":{"kind":"key","key":"tab","shell":"bad"}}"#,
        ] {
            assert!(parse_proposal(s).is_err());
        }
    }
    #[test]
    fn durable_intent_precedes_effect_and_blocks_replay() {
        let path = path();
        let approval = || {
            ReviewedAction::approve(
                Action::Key {
                    key: james_platform::Key::Tab,
                },
                1920,
                1080,
            )
            .unwrap()
        };
        execute_with(approval(), &path, &Cancellation::default(), |_, _| {
            assert!(needs_reconciliation(&path));
            assert!(std::fs::read_to_string(&path).unwrap().contains("tab"));
            Ok(())
        })
        .unwrap();
        assert!(
            execute_with(approval(), &path, &Cancellation::default(), |_, _| panic!(
                "must not replay"
            ))
            .is_err()
        );
        acknowledge(&path).unwrap();
        assert!(!needs_reconciliation(&path));
        let _ = std::fs::remove_dir(path.parent().unwrap());
    }
    #[test]
    fn cancellation_never_reaches_executor() {
        let path = path();
        let cancel = Cancellation::default();
        cancel.cancel();
        let approval = ReviewedAction::approve(
            Action::Key {
                key: james_platform::Key::Tab,
            },
            1920,
            1080,
        )
        .unwrap();
        assert!(execute_with(approval, &path, &cancel, |_, _| panic!("cancelled")).is_err());
        assert!(!path.exists());
    }
    #[test]
    fn failed_input_remains_uncertain_until_acknowledged() {
        let path = path();
        let approval = ReviewedAction::approve(Action::Click { x: 1, y: 1 }, 100, 100).unwrap();
        assert!(
            execute_with(approval, &path, &Cancellation::default(), |_, _| Err(
                james_platform::Error::TimedOut
            ))
            .is_err()
        );
        assert!(needs_reconciliation(&path));
        acknowledge(&path).unwrap();
        let _ = std::fs::remove_dir(path.parent().unwrap());
    }
}
