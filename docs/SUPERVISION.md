# The James supervision contract

**Implemented:** dependency-free, in-memory action coordination in `james-core`.  
**Not implemented:** database persistence, tool execution, credential handling, OS sandboxing, or a real user-approval UI.

## What the Rust supervisor owns

`RunSupervisor` holds a run-bound `Budget`, generates action IDs as `(run_id, ordinal)`, and emits ordered `RunEvent` entries containing only an action ID, state, and sequence. It cannot mint consent itself. An actual trusted user interface must call `Approval::after_user_review` after presenting the immutable invocation.

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
                                              host tool executor
                                              (NOT IMPLEMENTED)
```

`dispatch_with_journal(id, policy, time, recorder)` preflights the permission and run budget, calls the supplied recorder with the exact invocation, and only on recorder success consumes approval and reserves dispatch. A recorder failure leaves the action ready and the budget unchanged. The code rechecks permission and budget after the callback before reporting a running state.

**The recorder must be a real crash-durable, idempotent transaction keyed by action ID.** The included example uses a memory vector solely to demonstrate call ordering. Returning `Ok(())` from a closure is not evidence of durability, and `Action::mark_dispatched` remains a public low-level primitive. An incorrect host can bypass this high-level path, so no system-wide guarantee is claimed.

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

A real `james-store` SQLite journal, a single-writer runtime, deterministic replay from a persisted action ledger, and one read-only workspace tool with path/symlink/reparse-point checks. Exercise injected crashes around every intent/effect/outcome boundary before enabling file writes, desktop control, or network disclosures.

See [architecture](ARCHITECTURE.md), [roadmap](ROADMAP.md), and [security scope](../SECURITY.md).
