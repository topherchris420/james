//! In-memory run supervision. No executor, storage, or user interface lives
//! here. The caller supplies an actual durable intent recorder before dispatch.

use std::collections::BTreeMap;

use crate::{Action, Approval, Budget, Error, Invocation, Policy, State};

/// Assigned by the host-owned supervisor, never by a model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActionId {
    run_id: u64,
    ordinal: u64,
}

impl ActionId {
    pub fn run_id(self) -> u64 {
        self.run_id
    }

    pub fn ordinal(self) -> u64 {
        self.ordinal
    }
}

/// An intentionally minimal, secret-free execution timeline entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunEvent {
    pub sequence: u64,
    pub action_id: ActionId,
    pub state: State,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupervisorError {
    Core(Error),
    UnknownAction,
    WrongRun,
    IdExhausted,
    EventOverflow,
    JournalRejected,
}

impl From<Error> for SupervisorError {
    fn from(error: Error) -> Self {
        Self::Core(error)
    }
}

impl std::fmt::Display for SupervisorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for SupervisorError {}

/// Owns unique action identities, a single run budget and a state-only event
/// timeline. This is deliberately neither Clone nor a persistent journal.
#[derive(Debug)]
pub struct RunSupervisor {
    run_id: u64,
    next_action: u64,
    next_event: u64,
    actions: BTreeMap<ActionId, Action>,
    budget: Budget,
    events: Vec<RunEvent>,
}

impl RunSupervisor {
    pub fn new(
        run_id: u64,
        max_dispatches: u32,
        max_tokens: u64,
        deadline_ms: u64,
    ) -> Result<Self, SupervisorError> {
        if run_id == 0 {
            return Err(Error::InvalidInvocation.into());
        }
        Ok(Self {
            run_id,
            next_action: 1,
            next_event: 1,
            actions: BTreeMap::new(),
            budget: Budget::new(run_id, max_dispatches, max_tokens, deadline_ms),
            events: Vec::new(),
        })
    }

    pub fn propose(&mut self, invocation: Invocation) -> Result<ActionId, SupervisorError> {
        if invocation.run_id() != self.run_id {
            return Err(SupervisorError::WrongRun);
        }
        self.check_event_capacity()?;
        let following = self
            .next_action
            .checked_add(1)
            .ok_or(SupervisorError::IdExhausted)?;
        let id = ActionId {
            run_id: self.run_id,
            ordinal: self.next_action,
        };
        self.next_action = following;
        self.actions.insert(id, Action::new(invocation));
        self.record(id, State::Proposed);
        Ok(id)
    }

    pub fn action(&self, id: ActionId) -> Option<&Action> {
        self.actions.get(&id)
    }

    pub fn events(&self) -> &[RunEvent] {
        &self.events
    }

    pub fn usage(&self) -> (u32, u64) {
        self.budget.usage()
    }

    pub fn charge_tokens(&mut self, tokens: u64, now_ms: u64) -> Result<(), SupervisorError> {
        self.budget.charge_tokens(self.run_id, tokens, now_ms)?;
        Ok(())
    }

    pub fn prepare(&mut self, id: ActionId, policy: Policy) -> Result<State, SupervisorError> {
        self.check_event_capacity()?;
        let state = self.get_mut(id)?.prepare(policy)?;
        self.record(id, state);
        Ok(state)
    }

    /// Approval tokens are minted only by the trusted host after real review.
    /// The supervisor intentionally cannot issue consent to itself.
    pub fn approve(
        &mut self,
        id: ActionId,
        approval: Approval,
        now_ms: u64,
    ) -> Result<(), SupervisorError> {
        self.check_event_capacity()?;
        let action = self.get_mut(id)?;
        action.approve(approval, now_ms)?;
        self.record(id, State::Ready);
        Ok(())
    }

    pub fn deny(&mut self, id: ActionId) -> Result<(), SupervisorError> {
        self.check_event_capacity()?;
        self.get_mut(id)?.deny()?;
        self.record(id, State::Denied);
        Ok(())
    }

    /// Persist the concrete invocation before reserving dispatch, then recheck
    /// permission and quota. A failed recorder does not consume authorization
    /// or budget. The recorder must be crash-durable and idempotent by ActionId.
    ///
    /// On success the host may perform the actual side effect. The supervisor
    /// cannot verify that the closure persisted data or constrain the executor.
    pub fn dispatch_with_journal(
        &mut self,
        id: ActionId,
        policy: Policy,
        now_ms: u64,
        record_intent: impl FnOnce(ActionId, &Invocation) -> Result<(), ()>,
    ) -> Result<(), SupervisorError> {
        self.check_event_capacity()?;
        {
            let action = self
                .actions
                .get(&id)
                .ok_or(SupervisorError::UnknownAction)?;
            action.can_dispatch(policy, &self.budget, now_ms)?;
            record_intent(id, action.invocation())
                .map_err(|()| SupervisorError::JournalRejected)?;
        }
        let action = self
            .actions
            .get_mut(&id)
            .ok_or(SupervisorError::UnknownAction)?;
        action.mark_dispatched(policy, &mut self.budget, now_ms)?;
        self.record(id, State::Running);
        Ok(())
    }

    pub fn complete(&mut self, id: ActionId, succeeded: bool) -> Result<(), SupervisorError> {
        self.check_event_capacity()?;
        let action = self.get_mut(id)?;
        action.complete(succeeded)?;
        let state = action.state();
        self.record(id, state);
        Ok(())
    }

    pub fn cancel(&mut self, id: ActionId) -> Result<(), SupervisorError> {
        self.check_event_capacity()?;
        let action = self.get_mut(id)?;
        action.cancel()?;
        let state = action.state();
        self.record(id, state);
        Ok(())
    }

    pub fn confirm_cancelled(&mut self, id: ActionId) -> Result<(), SupervisorError> {
        self.check_event_capacity()?;
        self.get_mut(id)?.confirm_cancelled()?;
        self.record(id, State::Cancelled);
        Ok(())
    }

    /// A live supervisor can simulate recovery transitions for testing; the
    /// real host reconstructs a fresh supervisor from a durable event journal.
    pub fn recover_after_restart(&mut self, id: ActionId) -> Result<(), SupervisorError> {
        self.check_event_capacity()?;
        let action = self.get_mut(id)?;
        action.recover_after_restart();
        let state = action.state();
        self.record(id, state);
        Ok(())
    }

    pub fn reconcile(&mut self, id: ActionId, succeeded: bool) -> Result<(), SupervisorError> {
        self.check_event_capacity()?;
        let action = self.get_mut(id)?;
        action.reconcile(succeeded)?;
        let state = action.state();
        self.record(id, state);
        Ok(())
    }

    fn get_mut(&mut self, id: ActionId) -> Result<&mut Action, SupervisorError> {
        self.actions
            .get_mut(&id)
            .ok_or(SupervisorError::UnknownAction)
    }

    fn check_event_capacity(&self) -> Result<(), SupervisorError> {
        self.next_event
            .checked_add(1)
            .ok_or(SupervisorError::EventOverflow)?;
        Ok(())
    }

    fn record(&mut self, id: ActionId, state: State) {
        self.events.push(RunEvent {
            sequence: self.next_event,
            action_id: id,
            state,
        });
        self.next_event += 1;
    }
}
