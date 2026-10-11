//! Phase 1 executable contracts. This crate executes no models or desktop tools.
//! The trusted host must validate arguments, classify effects, enforce scopes,
//! persist intent before dispatch, and reconcile outcomes after a crash.

pub mod supervisor;

use std::collections::BTreeMap;
use std::sync::{Arc, Weak};

/// Classify using the trusted tool registry, never model/server annotations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    WorkspaceRead,
    FileWrite,
    DesktopControl,
    NetworkDisclosure,
    ExternalCommit,
}

/// An immutable, host-validated invocation. Exact equality binds the approval
/// to every field. This is an in-memory contract, not a signed authorization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    run_id: u64,
    tool: String,
    tool_identity: String,
    arguments: BTreeMap<String, String>,
    scope: String,
    destination: String,
    effect: Effect,
}

impl Invocation {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        run_id: u64,
        tool: impl Into<String>,
        tool_identity: impl Into<String>,
        arguments: BTreeMap<String, String>,
        scope: impl Into<String>,
        destination: impl Into<String>,
        effect: Effect,
    ) -> Result<Self, Error> {
        let invocation = Self {
            run_id,
            tool: tool.into(),
            tool_identity: tool_identity.into(),
            arguments,
            scope: scope.into(),
            destination: destination.into(),
            effect,
        };
        if run_id == 0
            || invocation.tool.trim().is_empty()
            || invocation.tool_identity.trim().is_empty()
            || invocation.scope.trim().is_empty()
            || invocation.destination.trim().is_empty()
        {
            return Err(Error::InvalidInvocation);
        }
        Ok(invocation)
    }

    pub fn run_id(&self) -> u64 {
        self.run_id
    }

    pub fn tool(&self) -> &str {
        &self.tool
    }

    pub fn tool_identity(&self) -> &str {
        &self.tool_identity
    }

    pub fn scope(&self) -> &str {
        &self.scope
    }

    pub fn destination(&self) -> &str {
        &self.destination
    }

    pub fn arguments(&self) -> &BTreeMap<String, String> {
        &self.arguments
    }

    pub fn effect(&self) -> Effect {
        self.effect
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Policy {
    /// Host increments on any permission, provider, extension or scope change.
    pub revision: u64,
    pub allow_workspace_reads: bool,
    pub tools_enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    AllowRead,
    Ask,
    Deny,
}

impl Policy {
    pub fn decide(&self, invocation: &Invocation) -> Decision {
        if !self.tools_enabled {
            Decision::Deny
        } else if self.allow_workspace_reads && invocation.effect == Effect::WorkspaceRead {
            Decision::AllowRead
        } else {
            Decision::Ask
        }
    }
}

/// One-use consent. Created only by a trusted host after concrete UI review.
/// It is intentionally neither Clone nor publicly constructible.
#[derive(Debug)]
pub struct Approval {
    invocation: Invocation,
    policy: Policy,
    action_generation: Weak<()>,
    expires_at_ms: u64,
}

impl Approval {
    pub fn after_user_review(
        action: &Action,
        policy: Policy,
        now_ms: u64,
        expires_at_ms: u64,
    ) -> Result<Self, Error> {
        if policy.decide(&action.invocation) == Decision::Deny {
            return Err(Error::Denied);
        }
        if action.state != State::AwaitingApproval {
            return Err(Error::InvalidTransition);
        }
        if expires_at_ms <= now_ms {
            return Err(Error::ApprovalExpired);
        }
        Ok(Self {
            invocation: action.invocation.clone(),
            policy,
            action_generation: Arc::downgrade(&action.generation),
            expires_at_ms,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Proposed,
    AwaitingApproval,
    Ready,
    Running,
    CancellationRequested,
    NeedsReconciliation,
    Succeeded,
    Failed,
    Denied,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    InvalidInvocation,
    InvalidTransition,
    Denied,
    ApprovalMismatch,
    ApprovalExpired,
    PolicyChanged,
    BudgetExceeded,
    DeadlineExceeded,
    CounterOverflow,
    RunMismatch,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for Error {}

/// Run-level ceiling; charge model tokens as they arrive and reserve each
/// dispatch before executing it. All times use one monotonic host clock.
#[derive(Debug, PartialEq, Eq)]
pub struct Budget {
    run_id: u64,
    max_dispatches: u32,
    max_tokens: u64,
    deadline_ms: u64,
    dispatches: u32,
    tokens: u64,
}

impl Budget {
    pub fn new(run_id: u64, max_dispatches: u32, max_tokens: u64, deadline_ms: u64) -> Self {
        Self {
            run_id,
            max_dispatches,
            max_tokens,
            deadline_ms,
            dispatches: 0,
            tokens: 0,
        }
    }

    fn check_deadline(&self, now_ms: u64) -> Result<(), Error> {
        if now_ms >= self.deadline_ms {
            Err(Error::DeadlineExceeded)
        } else {
            Ok(())
        }
    }

    pub fn charge_tokens(&mut self, run_id: u64, tokens: u64, now_ms: u64) -> Result<(), Error> {
        if run_id != self.run_id {
            return Err(Error::RunMismatch);
        }
        self.check_deadline(now_ms)?;
        let next = self
            .tokens
            .checked_add(tokens)
            .ok_or(Error::CounterOverflow)?;
        if next > self.max_tokens {
            return Err(Error::BudgetExceeded);
        }
        self.tokens = next;
        Ok(())
    }

    /// Check capacity without charging it. A journal adapter can use this
    /// before writing a durable intent and recheck at dispatch.
    pub fn can_reserve_dispatch(&self, run_id: u64, now_ms: u64) -> Result<(), Error> {
        if run_id != self.run_id {
            return Err(Error::RunMismatch);
        }
        self.check_deadline(now_ms)?;
        let next = self
            .dispatches
            .checked_add(1)
            .ok_or(Error::CounterOverflow)?;
        if next > self.max_dispatches {
            Err(Error::BudgetExceeded)
        } else {
            Ok(())
        }
    }

    fn reserve_dispatch(&mut self, run_id: u64, now_ms: u64) -> Result<(), Error> {
        self.can_reserve_dispatch(run_id, now_ms)?;
        self.dispatches += 1;
        Ok(())
    }

    pub fn usage(&self) -> (u32, u64) {
        (self.dispatches, self.tokens)
    }
}

/// One concrete action, within a supervisor-owned run. No setter can mutate a
/// reviewed invocation. Changed arguments require a new action and review.
#[derive(Debug)]
pub struct Action {
    invocation: Invocation,
    state: State,
    prepared_policy: Option<Policy>,
    approval: Option<Approval>,
    generation: Arc<()>,
}

impl Action {
    pub fn new(invocation: Invocation) -> Self {
        Self {
            invocation,
            state: State::Proposed,
            prepared_policy: None,
            approval: None,
            generation: Arc::new(()),
        }
    }

    pub fn invocation(&self) -> &Invocation {
        &self.invocation
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn prepare(&mut self, policy: Policy) -> Result<State, Error> {
        if self.state != State::Proposed {
            return Err(Error::InvalidTransition);
        }
        self.prepared_policy = Some(policy);
        self.state = match policy.decide(&self.invocation) {
            Decision::AllowRead => State::Ready,
            Decision::Ask => State::AwaitingApproval,
            Decision::Deny => State::Denied,
        };
        Ok(self.state)
    }

    pub fn approve(&mut self, approval: Approval, now_ms: u64) -> Result<(), Error> {
        if self.state != State::AwaitingApproval {
            return Err(Error::InvalidTransition);
        }
        if approval.invocation != self.invocation {
            return Err(Error::ApprovalMismatch);
        }
        if !approval
            .action_generation
            .upgrade()
            .is_some_and(|generation| Arc::ptr_eq(&generation, &self.generation))
        {
            return Err(Error::ApprovalMismatch);
        }
        if Some(approval.policy) != self.prepared_policy {
            return Err(Error::PolicyChanged);
        }
        if now_ms >= approval.expires_at_ms {
            return Err(Error::ApprovalExpired);
        }
        self.approval = Some(approval);
        self.state = State::Ready;
        Ok(())
    }

    pub fn deny(&mut self) -> Result<(), Error> {
        if self.state != State::AwaitingApproval {
            return Err(Error::InvalidTransition);
        }
        self.state = State::Denied;
        Ok(())
    }

    /// Read-only admission check before the host journals an intent. This is
    /// not authorization to execute: call mark_dispatched again after the
    /// durable record exists, and before the actual side effect.
    pub fn can_dispatch(&self, policy: Policy, budget: &Budget, now_ms: u64) -> Result<(), Error> {
        if self.state != State::Ready {
            return Err(Error::InvalidTransition);
        }
        if Some(policy) != self.prepared_policy {
            return Err(Error::PolicyChanged);
        }
        match policy.decide(&self.invocation) {
            Decision::Deny => return Err(Error::Denied),
            Decision::Ask => {
                let approval = self.approval.as_ref().ok_or(Error::ApprovalMismatch)?;
                if now_ms >= approval.expires_at_ms {
                    return Err(Error::ApprovalExpired);
                }
            }
            Decision::AllowRead => {}
        }
        budget.can_reserve_dispatch(self.invocation.run_id, now_ms)
    }

    /// Validates and consumes authorization immediately before dispatch.
    /// The host MUST durably journal this intent before an external side effect.
    /// This contract alone provides no persistence or security sandbox.
    pub fn mark_dispatched(
        &mut self,
        policy: Policy,
        budget: &mut Budget,
        now_ms: u64,
    ) -> Result<(), Error> {
        if self.state != State::Ready {
            return Err(Error::InvalidTransition);
        }
        if Some(policy) != self.prepared_policy {
            self.approval = None;
            self.state = State::Proposed;
            return Err(Error::PolicyChanged);
        }
        match policy.decide(&self.invocation) {
            Decision::Deny => {
                self.approval = None;
                self.state = State::Denied;
                return Err(Error::Denied);
            }
            Decision::Ask => {
                let approval = self.approval.as_ref().ok_or(Error::ApprovalMismatch)?;
                if now_ms >= approval.expires_at_ms {
                    self.approval = None;
                    self.state = State::AwaitingApproval;
                    return Err(Error::ApprovalExpired);
                }
            }
            Decision::AllowRead => {}
        }
        budget.reserve_dispatch(self.invocation.run_id, now_ms)?;
        self.approval = None;
        self.state = State::Running;
        Ok(())
    }

    /// A trustworthy tool result, not the model's statement of success.
    pub fn complete(&mut self, succeeded: bool) -> Result<(), Error> {
        if !matches!(self.state, State::Running | State::CancellationRequested) {
            return Err(Error::InvalidTransition);
        }
        self.state = if succeeded {
            State::Succeeded
        } else {
            State::Failed
        };
        Ok(())
    }

    pub fn cancel(&mut self) -> Result<(), Error> {
        match self.state {
            State::Proposed | State::AwaitingApproval | State::Ready => {
                self.approval = None;
                self.state = State::Cancelled;
            }
            State::Running => self.state = State::CancellationRequested,
            _ => return Err(Error::InvalidTransition),
        }
        Ok(())
    }

    /// Only report cancellation after the host confirms no side effect remains
    /// unresolved. Process termination by itself is insufficient confirmation.
    pub fn confirm_cancelled(&mut self) -> Result<(), Error> {
        if self.state != State::CancellationRequested {
            return Err(Error::InvalidTransition);
        }
        self.state = State::Cancelled;
        Ok(())
    }

    /// Revokes ephemeral approvals. In-flight actions have unknown outcomes and
    /// must not be retried automatically. The caller owns durable reconstruction.
    pub fn recover_after_restart(&mut self) {
        self.approval = None;
        self.prepared_policy = None;
        self.generation = Arc::new(());
        self.state = match self.state {
            State::Running | State::CancellationRequested => State::NeedsReconciliation,
            State::Ready | State::AwaitingApproval => State::Proposed,
            other => other,
        };
    }

    /// Explicit external reconciliation closes uncertainty; never authorizes
    /// repetition of the old operation.
    pub fn reconcile(&mut self, succeeded: bool) -> Result<(), Error> {
        if self.state != State::NeedsReconciliation {
            return Err(Error::InvalidTransition);
        }
        self.state = if succeeded {
            State::Succeeded
        } else {
            State::Failed
        };
        Ok(())
    }
}
