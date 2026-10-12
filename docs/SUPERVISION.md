# The James supervision contract

**Implemented:** dependency-free, in-memory action coordination in `james-core`, and one host that uses it: the desktop's reviewed-input path on Windows (one click, key or text), with an Allow once review screen and a flushed, single-record intent file.  
**Not implemented:** a general tool executor, a database or multi-record ledger, credential handling, OS sandboxing, or automatic verification of an action's outcome.

## What the Rust supervisor owns

`RunSupervisor` holds a run-bound `Budget`, generates action IDs as `(run_id, ordinal)`, and emits ordered `RunEvent` entries containing only an action ID, state, and sequence. It cannot mint consent itself. A trusted user interface must call `Approval::after_user_review` after presenting the immutable invocation; in James that is only the desktop's **Allow once** button, which hands the worker a non-cloneable `ReviewedAction`.

A model response is never authority. Tool identity, effect category, destination and scope must be validated by the trusted host, not taken from untrusted model/server declarations.

## Intent before execution

```text
propose -> prepare -> trusted review -> approve
                                      |
                            read-only can_dispatch()
                              |               |
                            reject          allowed
                                              |
                                    record_intent callback
                                      |               |
                                  failure           success
                                      |               |
                                  no dispatch   mark_dispatched()
                                                       |
                                              host executor
                                    (james-desktop: one Windows input)
```

`dispatch_with_journal(id, policy, time, recorder)` preflights the permission and run budget, calls the supplied recorder with the exact invocation, and only on recorder success consumes approval and reserves dispatch. A recorder failure leaves the action ready and the budget unchanged. The code rechecks permission and budget after the callback before reporting a running state.

**The recorder must be a real crash-durable, idempotent transaction keyed by action ID.** The included example uses a memory vector solely to demonstrate call ordering. Returning `Ok(())` from a closure is not evidence of durability, and `Action::mark_dispatched` remains a public low-level primitive. An incorrect host can bypass this high-level path, so no system-wide guarantee is claimed.

## The desktop host

`crates/james-desktop/src/review.rs` runs every approved input through this path with a budget of one action. Its recorder writes `action-intent.json` with the record's id, time, exact action, screen size and target window, using `create_new` (so it fails while an older record is unresolved), `sync_all`, and on Unix a folder sync and mode 0600. Input is sent only after that write succeeds and the approval, cancellation and target-window checks pass again. The outcome is then added to the record by an atomic replacement, and the record is removed only when the person confirms they checked the target application.

| Crash or failure window | James desktop 0.4 |
| --- | --- |
| Before the intent record is written | Nothing is sent. The approval lives only in memory and is gone after a restart |
| After the record, before input | The record survives with no outcome. On the next launch James shows the action and says the outcome is unknown; new actions stay blocked until the person checks |
| During input | Same as above: unknown, shown, never replayed |
| After input, before the outcome is written | Same: the person, not James, decides what happened |
| After the outcome is written | The record shows "sent", "not sent" and why, or the helper's error, until acknowledged |
| Recorder fails | No input, no budget used; the person is told to inspect any earlier record |

This is human reconciliation, not a transaction: James cannot observe whether a click had its intended effect, and an acknowledgement is the person's judgement, not proof. The automated tests in `review.rs` cover each window; a real crash during a real Windows input has not been exercised (see [END_USER_READINESS.md](END_USER_READINESS.md)).

## What the next host must prove

| Crash/failure window | Required treatment |
| --- | --- |
| Before durable intent commit | No external tool may run; revoke transient consent after restart |
| After intent commit, before dispatch | Reconstruct the pending record; do not assume the side effect did or did not run |
| During an external side effect | Outcome is **unknown**; block automatic replay |
| After side effect, before outcome commit | Verify with independent external evidence or an idempotency key |
| After durable outcome commit | Reconstruct terminal status without executing again |
| Recorder returns failure | No dispatch, no budget consumed; retry only after verifying intent state |

The in-memory `recover_after_restart` helper exercises state transitions but cannot rebuild a real process from persisted data. The event vector is not a durable audit log and intentionally omits arguments, prompt text, and secrets.

## Reproducing the contract

```bash
cargo test --workspace --all-targets --locked --offline
cargo run --locked --offline --example supervised_demo
```

Adversarial tests cover separate identical actions receiving different IDs, shared budget exhaustion, failed journal callbacks, expired permission, changed policy, interrupted execution, cancellation, cross-run rejection, and token budget enforcement.

## Next practical vertical slice

A real `james-store` SQLite journal, a single-writer runtime, deterministic replay from a persisted action ledger, and one read-only workspace tool with path/symlink/reparse-point checks. Exercise injected crashes around every intent/effect/outcome boundary before enabling file writes, multi-step desktop control, input on macOS or Linux, or network disclosures.

See [architecture](ARCHITECTURE.md), [roadmap](ROADMAP.md), and [security scope](../SECURITY.md).
