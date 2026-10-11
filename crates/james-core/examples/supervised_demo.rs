//! Simulated supervision. The vector is not a durable journal and no tool runs.
use james_core::supervisor::RunSupervisor;
use james_core::{Approval, Effect, Invocation, Policy};
use std::collections::BTreeMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let policy = Policy {
        revision: 1,
        allow_workspace_reads: true,
        tools_enabled: true,
    };
    let mut run = RunSupervisor::new(1, 1, 128, 100)?;
    let request = Invocation::new(
        1,
        "file.write",
        "builtin:v1",
        BTreeMap::from([("path".into(), "notes.txt".into())]),
        "workspace:demo",
        "device",
        Effect::FileWrite,
    )?;
    let id = run.propose(request)?;
    println!("Action {}: {:?}", id.ordinal(), run.prepare(id, policy)?);
    // Simulated consent, NOT a real human authorization prompt.
    let approval = Approval::after_user_review(run.action(id).unwrap(), policy, 0, 50)?;
    run.approve(id, approval, 1)?;

    let mut simulated_intents = Vec::new();
    run.dispatch_with_journal(id, policy, 2, |id, invocation| {
        simulated_intents.push((id, invocation.tool().to_owned()));
        Ok(())
    })?;
    println!("In-memory intent count: {}", simulated_intents.len());
    run.recover_after_restart(id)?;
    run.reconcile(id, true)?;
    for event in run.events() {
        println!(
            "#{:02} action={} state={:?}",
            event.sequence,
            event.action_id.ordinal(),
            event.state
        );
    }
    println!("No external action executed; nothing was persisted.");
    Ok(())
}
