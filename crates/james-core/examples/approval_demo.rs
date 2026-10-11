use james_core::{Action, Approval, Budget, Effect, Invocation, Policy};
use std::collections::BTreeMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("James Phase 1 contract demonstration; no file or desktop action executes.");
    let policy = Policy {
        revision: 1,
        allow_workspace_reads: true,
        tools_enabled: true,
    };
    let inv = Invocation::new(
        1,
        "file.write",
        "builtin:v1",
        BTreeMap::from([("path".into(), "draft.txt".into())]),
        "workspace:demo",
        "device",
        Effect::FileWrite,
    )?;
    let mut action = Action::new(inv);
    println!("Prepared: {:?}", action.prepare(policy)?);
    // Simulated trusted UI consent, not an actual user authorization prompt.
    let consent = Approval::after_user_review(&action, policy, 0, 100)?;
    action.approve(consent, 1)?;
    let mut budget = Budget::new(1, 1, 128, 100);
    action.mark_dispatched(policy, &mut budget, 2)?;
    println!("Simulated dispatch: {:?}", action.state());
    action.recover_after_restart();
    println!("Simulated interrupted operation: {:?}", action.state());
    println!(
        "Replay blocked: {:?}",
        action.mark_dispatched(policy, &mut budget, 3)
    );
    action.reconcile(true)?;
    println!(
        "Explicitly reconciled: {:?}; budget: {:?}",
        action.state(),
        budget.usage()
    );
    Ok(())
}
