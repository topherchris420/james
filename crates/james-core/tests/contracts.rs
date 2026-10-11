use james_core::{Action, Approval, Budget, Decision, Effect, Error, Invocation, Policy, State};
use std::collections::BTreeMap;

#[test]
fn approval_cannot_transfer_to_a_separate_identical_action() {
    let inv = invocation(Effect::FileWrite);
    let mut first = Action::new(inv.clone());
    let mut second = Action::new(inv);
    first.prepare(policy()).unwrap();
    second.prepare(policy()).unwrap();
    let token = Approval::after_user_review(&first, policy(), 0, 100).unwrap();
    assert_eq!(second.approve(token, 1), Err(Error::ApprovalMismatch));
}

#[test]
fn outstanding_approval_is_invalid_after_recovery() {
    let mut action = Action::new(invocation(Effect::FileWrite));
    action.prepare(policy()).unwrap();
    let token = Approval::after_user_review(&action, policy(), 0, 100).unwrap();
    action.recover_after_restart();
    action.prepare(policy()).unwrap();
    assert_eq!(action.approve(token, 1), Err(Error::ApprovalMismatch));
}

#[test]
fn full_policy_snapshot_detects_a_missing_revision_increment() {
    let mut action = Action::new(invocation(Effect::WorkspaceRead));
    action.prepare(policy()).unwrap();
    let changed = Policy {
        allow_workspace_reads: false,
        ..policy()
    };
    assert_eq!(
        action.mark_dispatched(changed, &mut Budget::new(1, 1, 10, 100), 1),
        Err(Error::PolicyChanged)
    );
    assert_eq!(action.state(), State::Proposed);
    action.prepare(changed).unwrap();
    assert_eq!(action.state(), State::AwaitingApproval);
    let token = Approval::after_user_review(&action, changed, 1, 100).unwrap();
    action.approve(token, 2).unwrap();
    action
        .mark_dispatched(changed, &mut Budget::new(1, 1, 10, 100), 3)
        .unwrap();
}

#[test]
fn budget_cannot_be_used_by_another_run() {
    let mut budget = Budget::new(2, 1, 10, 100);
    assert_eq!(budget.charge_tokens(1, 1, 1), Err(Error::RunMismatch));
    assert_eq!(
        approved_action().mark_dispatched(policy(), &mut budget, 2),
        Err(Error::RunMismatch)
    );
    assert_eq!(budget.usage(), (0, 0));
}

fn policy() -> Policy {
    Policy {
        revision: 1,
        allow_workspace_reads: true,
        tools_enabled: true,
    }
}

fn invocation(effect: Effect) -> Invocation {
    Invocation::new(
        1,
        "file.write",
        "builtin:v1",
        BTreeMap::from([("path".into(), "draft.txt".into())]),
        "workspace:fixture",
        "device",
        effect,
    )
    .unwrap()
}

fn approved_action() -> Action {
    let mut action = Action::new(invocation(Effect::FileWrite));
    action.prepare(policy()).unwrap();
    let consent = Approval::after_user_review(&action, policy(), 0, 100).unwrap();
    action.approve(consent, 1).unwrap();
    action
}

#[test]
fn every_effect_requires_review_except_enabled_workspace_reads() {
    for effect in [
        Effect::FileWrite,
        Effect::DesktopControl,
        Effect::NetworkDisclosure,
        Effect::ExternalCommit,
    ] {
        assert_eq!(policy().decide(&invocation(effect)), Decision::Ask);
    }
    assert_eq!(
        policy().decide(&invocation(Effect::WorkspaceRead)),
        Decision::AllowRead
    );
    assert_eq!(
        Policy {
            allow_workspace_reads: false,
            ..policy()
        }
        .decide(&invocation(Effect::WorkspaceRead)),
        Decision::Ask
    );
}

#[test]
fn disabled_tools_deny_all_effects_and_consent() {
    let disabled = Policy {
        tools_enabled: false,
        ..policy()
    };
    for effect in [
        Effect::WorkspaceRead,
        Effect::FileWrite,
        Effect::DesktopControl,
        Effect::NetworkDisclosure,
        Effect::ExternalCommit,
    ] {
        let mut action = Action::new(invocation(effect));
        assert_eq!(action.prepare(disabled), Ok(State::Denied));
        assert_eq!(
            Approval::after_user_review(&action, disabled, 0, 100).unwrap_err(),
            Error::Denied
        );
        assert_eq!(
            action.mark_dispatched(disabled, &mut Budget::new(1, 1, 1, 100), 1),
            Err(Error::InvalidTransition)
        );
    }
}

#[test]
fn invalid_identity_is_rejected() {
    assert_eq!(
        Invocation::new(
            0,
            "tool",
            "v1",
            BTreeMap::new(),
            "scope",
            "device",
            Effect::WorkspaceRead
        ),
        Err(Error::InvalidInvocation)
    );
    assert_eq!(
        Invocation::new(
            1,
            "  ",
            "v1",
            BTreeMap::new(),
            "scope",
            "device",
            Effect::WorkspaceRead
        ),
        Err(Error::InvalidInvocation)
    );
}

#[test]
fn cannot_dispatch_a_plan_or_unapproved_action() {
    let mut action = Action::new(invocation(Effect::ExternalCommit));
    let mut budget = Budget::new(1, 1, 10, 100);
    assert_eq!(
        action.mark_dispatched(policy(), &mut budget, 1),
        Err(Error::InvalidTransition)
    );
    action.prepare(policy()).unwrap();
    assert_eq!(
        action.mark_dispatched(policy(), &mut budget, 1),
        Err(Error::InvalidTransition)
    );
    assert_eq!(budget.usage(), (0, 0));
}

#[test]
fn review_is_bound_to_run_tool_identity_arguments_scope_destination_and_effect() {
    let original = invocation(Effect::FileWrite);
    let mut original_action = Action::new(original.clone());
    original_action.prepare(policy()).unwrap();
    let variants = [
        Invocation::new(
            2,
            "file.write",
            "builtin:v1",
            original.arguments().clone(),
            "workspace:fixture",
            "device",
            Effect::FileWrite,
        )
        .unwrap(),
        Invocation::new(
            1,
            "file.delete",
            "builtin:v1",
            original.arguments().clone(),
            "workspace:fixture",
            "device",
            Effect::FileWrite,
        )
        .unwrap(),
        Invocation::new(
            1,
            "file.write",
            "builtin:v2",
            original.arguments().clone(),
            "workspace:fixture",
            "device",
            Effect::FileWrite,
        )
        .unwrap(),
        Invocation::new(
            1,
            "file.write",
            "builtin:v1",
            BTreeMap::from([("path".into(), "other.txt".into())]),
            "workspace:fixture",
            "device",
            Effect::FileWrite,
        )
        .unwrap(),
        Invocation::new(
            1,
            "file.write",
            "builtin:v1",
            original.arguments().clone(),
            "workspace:other",
            "device",
            Effect::FileWrite,
        )
        .unwrap(),
        Invocation::new(
            1,
            "file.write",
            "builtin:v1",
            original.arguments().clone(),
            "workspace:fixture",
            "cloud",
            Effect::FileWrite,
        )
        .unwrap(),
        Invocation::new(
            1,
            "file.write",
            "builtin:v1",
            original.arguments().clone(),
            "workspace:fixture",
            "device",
            Effect::ExternalCommit,
        )
        .unwrap(),
    ];
    for changed in variants {
        let mut action = Action::new(changed);
        action.prepare(policy()).unwrap();
        let consent = Approval::after_user_review(&original_action, policy(), 0, 100).unwrap();
        assert_eq!(action.approve(consent, 1), Err(Error::ApprovalMismatch));
        assert_eq!(action.state(), State::AwaitingApproval);
    }
}

#[test]
fn consent_must_be_live_when_created_and_when_applied() {
    let inv = invocation(Effect::FileWrite);
    let mut action = Action::new(inv);
    action.prepare(policy()).unwrap();
    assert_eq!(
        Approval::after_user_review(&action, policy(), 100, 100).unwrap_err(),
        Error::ApprovalExpired
    );
    let consent = Approval::after_user_review(&action, policy(), 0, 100).unwrap();
    assert_eq!(action.approve(consent, 100), Err(Error::ApprovalExpired));
}

#[test]
fn expiry_between_review_and_dispatch_requires_new_consent() {
    let mut action = approved_action();
    let mut budget = Budget::new(1, 1, 10, 200);
    assert_eq!(
        action.mark_dispatched(policy(), &mut budget, 100),
        Err(Error::ApprovalExpired)
    );
    assert_eq!(action.state(), State::AwaitingApproval);
    assert_eq!(budget.usage(), (0, 0));
    let consent = Approval::after_user_review(&action, policy(), 100, 150).unwrap();
    action.approve(consent, 101).unwrap();
    action.mark_dispatched(policy(), &mut budget, 102).unwrap();
}

#[test]
fn changed_policy_at_dispatch_revokes_consent() {
    let mut action = approved_action();
    let changed = Policy {
        revision: 2,
        ..policy()
    };
    let mut budget = Budget::new(1, 1, 10, 200);
    assert_eq!(
        action.mark_dispatched(changed, &mut budget, 2),
        Err(Error::PolicyChanged)
    );
    assert_eq!(action.state(), State::Proposed);
    action.prepare(changed).unwrap();
    assert_eq!(action.state(), State::AwaitingApproval);
    assert_eq!(
        action.mark_dispatched(changed, &mut budget, 3),
        Err(Error::InvalidTransition)
    );
}

#[test]
fn approval_for_an_old_policy_cannot_be_applied() {
    let mut action = Action::new(invocation(Effect::FileWrite));
    action
        .prepare(Policy {
            revision: 2,
            ..policy()
        })
        .unwrap();
    let consent = Approval::after_user_review(&action, policy(), 0, 100).unwrap();
    assert_eq!(action.approve(consent, 1), Err(Error::PolicyChanged));
}

#[test]
fn live_disabled_policy_blocks_a_previously_ready_read() {
    let mut action = Action::new(invocation(Effect::WorkspaceRead));
    action.prepare(policy()).unwrap();
    assert_eq!(
        action.mark_dispatched(
            Policy {
                tools_enabled: false,
                ..policy()
            },
            &mut Budget::new(1, 1, 10, 100),
            1
        ),
        Err(Error::PolicyChanged)
    );
    assert_eq!(action.state(), State::Proposed);
    action
        .prepare(Policy {
            tools_enabled: false,
            ..policy()
        })
        .unwrap();
    assert_eq!(action.state(), State::Denied);
}

#[test]
fn dispatch_consumes_consent_and_cannot_be_replayed_on_the_action() {
    let mut action = approved_action();
    let mut budget = Budget::new(1, 5, 10, 100);
    action.mark_dispatched(policy(), &mut budget, 2).unwrap();
    assert_eq!(
        action.mark_dispatched(policy(), &mut budget, 3),
        Err(Error::InvalidTransition)
    );
    action.complete(true).unwrap();
    assert_eq!(
        action.mark_dispatched(policy(), &mut budget, 4),
        Err(Error::InvalidTransition)
    );
    assert_eq!(action.state(), State::Succeeded);
    assert_eq!(budget.usage(), (1, 0));
}

#[test]
fn exhausted_budget_does_not_dispatch_or_mutate_usage() {
    let mut action = approved_action();
    let mut budget = Budget::new(1, 0, 10, 100);
    assert_eq!(
        action.mark_dispatched(policy(), &mut budget, 2),
        Err(Error::BudgetExceeded)
    );
    assert_eq!(action.state(), State::Ready);
    assert_eq!(budget.usage(), (0, 0));
}

#[test]
fn shared_budget_limits_multiple_actions() {
    let mut budget = Budget::new(1, 1, 10, 100);
    approved_action()
        .mark_dispatched(policy(), &mut budget, 2)
        .unwrap();
    assert_eq!(
        approved_action().mark_dispatched(policy(), &mut budget, 3),
        Err(Error::BudgetExceeded)
    );
    assert_eq!(budget.usage(), (1, 0));
}

#[test]
fn deadline_is_exclusive_and_stops_tokens_and_dispatch() {
    let mut budget = Budget::new(1, 1, 10, 10);
    assert_eq!(budget.charge_tokens(1, 1, 10), Err(Error::DeadlineExceeded));
    assert_eq!(
        approved_action().mark_dispatched(policy(), &mut budget, 10),
        Err(Error::DeadlineExceeded)
    );
    assert_eq!(budget.usage(), (0, 0));
}

#[test]
fn token_budget_rejects_overflow_and_overrun_without_charging() {
    let mut budget = Budget::new(1, 1, 10, 100);
    budget.charge_tokens(1, 10, 1).unwrap();
    assert_eq!(budget.charge_tokens(1, 1, 2), Err(Error::BudgetExceeded));
    assert_eq!(budget.usage(), (0, 10));
    let mut huge = Budget::new(1, 1, u64::MAX, 100);
    huge.charge_tokens(1, u64::MAX, 1).unwrap();
    assert_eq!(huge.charge_tokens(1, 1, 2), Err(Error::CounterOverflow));
    assert_eq!(huge.usage(), (0, u64::MAX));
}

#[test]
fn restart_invalidates_approval_before_dispatch() {
    let mut action = approved_action();
    action.recover_after_restart();
    assert_eq!(action.state(), State::Proposed);
    action.prepare(policy()).unwrap();
    assert_eq!(action.state(), State::AwaitingApproval);
}

#[test]
fn interrupted_dispatch_has_unknown_outcome_and_no_automatic_retry() {
    for request_cancel in [false, true] {
        let mut action = approved_action();
        let mut budget = Budget::new(1, 5, 10, 100);
        action.mark_dispatched(policy(), &mut budget, 2).unwrap();
        if request_cancel {
            action.cancel().unwrap();
        }
        action.recover_after_restart();
        assert_eq!(action.state(), State::NeedsReconciliation);
        assert_eq!(
            action.mark_dispatched(policy(), &mut budget, 3),
            Err(Error::InvalidTransition)
        );
        assert_eq!(action.complete(false), Err(Error::InvalidTransition));
        action.reconcile(true).unwrap();
        assert_eq!(action.state(), State::Succeeded);
        assert_eq!(action.reconcile(false), Err(Error::InvalidTransition));
        assert_eq!(budget.usage(), (1, 0));
    }
}

#[test]
fn restart_preserves_terminal_outcome() {
    let mut action = approved_action();
    action
        .mark_dispatched(policy(), &mut Budget::new(1, 1, 10, 100), 2)
        .unwrap();
    action.complete(false).unwrap();
    action.recover_after_restart();
    assert_eq!(action.state(), State::Failed);
}

#[test]
fn cancel_before_dispatch_revokes_all_authority() {
    let mut action = approved_action();
    action.cancel().unwrap();
    assert_eq!(action.state(), State::Cancelled);
    assert_eq!(
        action.mark_dispatched(policy(), &mut Budget::new(1, 1, 10, 100), 2),
        Err(Error::InvalidTransition)
    );
    action.recover_after_restart();
    assert_eq!(action.state(), State::Cancelled);
}

#[test]
fn cancellation_intent_is_not_a_confirmed_outcome() {
    let mut action = approved_action();
    action
        .mark_dispatched(policy(), &mut Budget::new(1, 1, 10, 100), 2)
        .unwrap();
    action.cancel().unwrap();
    assert_eq!(action.state(), State::CancellationRequested);
    action.complete(true).unwrap();
    assert_eq!(action.state(), State::Succeeded);
}

#[test]
fn only_in_flight_cancellation_can_be_confirmed() {
    let mut action = approved_action();
    assert_eq!(action.confirm_cancelled(), Err(Error::InvalidTransition));
    action
        .mark_dispatched(policy(), &mut Budget::new(1, 1, 10, 100), 2)
        .unwrap();
    action.cancel().unwrap();
    action.confirm_cancelled().unwrap();
    assert_eq!(action.state(), State::Cancelled);
}

#[test]
fn denial_and_invalid_outcomes_cannot_bypass_dispatch() {
    let mut action = Action::new(invocation(Effect::ExternalCommit));
    assert_eq!(action.complete(true), Err(Error::InvalidTransition));
    assert_eq!(action.reconcile(true), Err(Error::InvalidTransition));
    action.prepare(policy()).unwrap();
    action.deny().unwrap();
    assert_eq!(action.state(), State::Denied);
    assert_eq!(action.prepare(policy()), Err(Error::InvalidTransition));
}
