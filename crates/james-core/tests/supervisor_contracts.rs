use james_core::supervisor::{RunSupervisor, SupervisorError};
use james_core::{Approval, Effect, Error, Invocation, Policy, State};
use std::collections::BTreeMap;

fn policy() -> Policy {
    Policy {
        revision: 1,
        allow_workspace_reads: true,
        tools_enabled: true,
    }
}

fn invocation(run_id: u64) -> Invocation {
    Invocation::new(
        run_id,
        "file.write",
        "builtin:v1",
        BTreeMap::from([("path".into(), "report.txt".into())]),
        "workspace:fixture",
        "device",
        Effect::FileWrite,
    )
    .unwrap()
}

#[test]
fn unique_ids_and_a_single_budget_for_identical_actions() {
    let mut run = RunSupervisor::new(7, 1, 10, 100).unwrap();
    let first = run.propose(invocation(7)).unwrap();
    let second = run.propose(invocation(7)).unwrap();
    assert_ne!(first, second);
    assert_eq!(first.run_id(), 7);
    assert_eq!(second.ordinal(), 2);

    for id in [first, second] {
        assert_eq!(run.prepare(id, policy()).unwrap(), State::AwaitingApproval);
        let consent =
            Approval::after_user_review(run.action(id).unwrap(), policy(), 0, 90).unwrap();
        run.approve(id, consent, 1).unwrap();
    }

    let mut recorded = Vec::new();
    run.dispatch_with_journal(first, policy(), 2, |id, inv| {
        recorded.push((id, inv.tool().to_owned()));
        Ok(())
    })
    .unwrap();
    assert_eq!(recorded, vec![(first, "file.write".to_owned())]);
    assert_eq!(run.usage(), (1, 0));

    let mut attempted = false;
    assert_eq!(
        run.dispatch_with_journal(second, policy(), 3, |_, _| {
            attempted = true;
            Ok(())
        }),
        Err(SupervisorError::Core(Error::BudgetExceeded))
    );
    assert!(
        !attempted,
        "do not journal a dispatch that cannot fit in budget"
    );
    assert_eq!(run.action(second).unwrap().state(), State::Ready);
}

#[test]
fn failed_journal_fails_closed_without_consuming_approval_or_budget() {
    let mut run = RunSupervisor::new(7, 1, 20, 100).unwrap();
    let id = run.propose(invocation(7)).unwrap();
    run.prepare(id, policy()).unwrap();
    let consent = Approval::after_user_review(run.action(id).unwrap(), policy(), 0, 90).unwrap();
    run.approve(id, consent, 1).unwrap();

    assert_eq!(
        run.dispatch_with_journal(id, policy(), 2, |_, _| Err(())),
        Err(SupervisorError::JournalRejected)
    );
    assert_eq!(run.usage(), (0, 0));
    assert_eq!(run.action(id).unwrap().state(), State::Ready);

    run.dispatch_with_journal(id, policy(), 3, |_, inv| {
        assert_eq!(inv.scope(), "workspace:fixture");
        Ok(())
    })
    .unwrap();
    assert_eq!(run.action(id).unwrap().state(), State::Running);
    run.complete(id, true).unwrap();
    assert_eq!(run.action(id).unwrap().state(), State::Succeeded);
}

#[test]
fn expired_or_changed_permissions_never_reach_journal() {
    let mut run = RunSupervisor::new(7, 1, 10, 200).unwrap();
    let id = run.propose(invocation(7)).unwrap();
    run.prepare(id, policy()).unwrap();
    let consent = Approval::after_user_review(run.action(id).unwrap(), policy(), 0, 50).unwrap();
    run.approve(id, consent, 1).unwrap();

    let mut recorded = false;
    assert_eq!(
        run.dispatch_with_journal(id, policy(), 50, |_, _| {
            recorded = true;
            Ok(())
        }),
        Err(SupervisorError::Core(Error::ApprovalExpired))
    );
    assert!(!recorded);

    let changed = Policy {
        revision: 2,
        ..policy()
    };
    assert_eq!(
        run.dispatch_with_journal(id, changed, 10, |_, _| {
            recorded = true;
            Ok(())
        }),
        Err(SupervisorError::Core(Error::PolicyChanged))
    );
    assert!(!recorded);
}

#[test]
fn journaled_interruption_requires_explicit_reconciliation() {
    let mut run = RunSupervisor::new(7, 2, 10, 100).unwrap();
    let id = run.propose(invocation(7)).unwrap();
    run.prepare(id, policy()).unwrap();
    let consent = Approval::after_user_review(run.action(id).unwrap(), policy(), 0, 90).unwrap();
    run.approve(id, consent, 1).unwrap();
    run.dispatch_with_journal(id, policy(), 2, |_, _| Ok(()))
        .unwrap();
    run.cancel(id).unwrap();
    assert_eq!(
        run.action(id).unwrap().state(),
        State::CancellationRequested
    );
    run.recover_after_restart(id).unwrap();
    assert_eq!(run.action(id).unwrap().state(), State::NeedsReconciliation);

    let mut attempted = false;
    assert_eq!(
        run.dispatch_with_journal(id, policy(), 3, |_, _| {
            attempted = true;
            Ok(())
        }),
        Err(SupervisorError::Core(Error::InvalidTransition))
    );
    assert!(!attempted);
    run.reconcile(id, true).unwrap();
    assert_eq!(run.action(id).unwrap().state(), State::Succeeded);
    assert_eq!(run.usage(), (1, 0));
}

#[test]
fn wrong_run_unknown_action_and_secret_free_timeline() {
    let mut run = RunSupervisor::new(7, 1, 10, 100).unwrap();
    assert_eq!(run.propose(invocation(8)), Err(SupervisorError::WrongRun));
    let id = run.propose(invocation(7)).unwrap();
    let state = run.prepare(id, policy()).unwrap();
    assert_eq!(state, State::AwaitingApproval);
    run.deny(id).unwrap();
    assert_eq!(
        run.events()
            .iter()
            .map(|event| event.state)
            .collect::<Vec<_>>(),
        vec![State::Proposed, State::AwaitingApproval, State::Denied]
    );
    assert_eq!(run.events()[0].sequence, 1);
    assert_eq!(run.events()[2].action_id, id);
    assert_eq!(run.usage(), (0, 0));
}

#[test]
fn cancellation_before_dispatch_never_calls_recorder() {
    let mut run = RunSupervisor::new(7, 2, 10, 100).unwrap();
    let id = run.propose(invocation(7)).unwrap();
    run.prepare(id, policy()).unwrap();
    run.cancel(id).unwrap();
    let mut recorded = false;
    assert_eq!(
        run.dispatch_with_journal(id, policy(), 2, |_, _| {
            recorded = true;
            Ok(())
        }),
        Err(SupervisorError::Core(Error::InvalidTransition))
    );
    assert!(!recorded);
}

#[test]
fn token_usage_is_charged_once_by_the_owner() {
    let mut run = RunSupervisor::new(7, 2, 4, 100).unwrap();
    run.charge_tokens(3, 1).unwrap();
    assert_eq!(
        run.charge_tokens(2, 2),
        Err(SupervisorError::Core(Error::BudgetExceeded))
    );
    assert_eq!(run.usage(), (0, 3));
}
