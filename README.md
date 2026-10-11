<div align="center">

# JAMES

### Intelligence proposes. You decide. James remembers what happened.

**An open-source, human-directed desktop AI assistant, being built in Rust.**

[![CI](https://github.com/topherchris420/james/actions/workflows/ci.yml/badge.svg)](https://github.com/topherchris420/james/actions/workflows/ci.yml)
![Rust](https://img.shields.io/badge/Rust-1.85%2B-0B5D63)
![MIT](https://img.shields.io/badge/license-MIT-0B5D63)
![Prototype](https://img.shields.io/badge/stage-core%20prototype-555555)

</div>

---

**What if a personal AI could be useful without quietly taking control?**

James is an experiment in a more deliberate relationship with artificial intelligence: one where a model can propose concrete actions, the human retains authority, and the system reports what actually happened, including what it *cannot* verify.

**Today, James is an executable Rust core and research project, not a usable desktop assistant.** There is no UI, model inference, voice, screen capture, MCP executor, persistent journal, or OS sandbox yet. The examples simulate approval and recovery without touching your files or desktop.

## One idea, four contracts

```text
       REQUEST
          │
          ▼
     AI proposes ─────► Immutable action
                              │
                       Human reviews
                              │
                      Policy + budget gate
                              │
                        Record intent
                              │
                      Trusted executor
                              │
                    Verify actual outcome
                              │
                           TIMELINE
```

1. **Consent is specific.** Approval is bound to one action instance, exact invocation, policy snapshot, and expiry. Edit the action and permission must be reconsidered.
2. **Budgets belong to the run.** A supervisor owns the quota and uniquely identifies each proposed action. Agents cannot grant themselves extra work.
3. **Intent precedes effect.** The supervisor checks authorization and quota before invoking an external intent-recording callback. Failed recording prevents dispatch.
4. **Uncertainty has a state.** Interrupted side effects are not automatically retried or labeled successful. They require reconciliation.

The first three are *in-memory contracts*. Real durability, user identity, path confinement, tool execution, and crash recovery still require a tested trusted host. A callback that says it persisted something is **not proof of persistence**.

## Run what exists

Requires Rust **1.85+** and a configured linker. `james-core` has no third-party Rust dependencies.

```bash
git clone https://github.com/topherchris420/james.git
cd james
cargo fmt --all -- --check
cargo test --workspace --all-targets --locked --offline
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
cargo run --locked --offline --example approval_demo
cargo run --locked --offline --example supervised_demo
```

The approval demo shows a simulated interrupted action that cannot be replayed. The supervisor demo shows distinct action identity, an in-memory intent callback, and a secret-free execution timeline. **Neither example performs an actual tool call or durable write.** On Windows, [`scripts/check.ps1`](scripts/check.ps1) supports local Rust/linker overrides. Check [validation](docs/VALIDATION.md) for measured results and platform scope.

## What exists and what is planned

| Capability | Implemented now | Planned |
| --- | --- | --- |
| Authorization | Immutable invocations, one-use approvals, policy revision checks | Actual keyboard-accessible review UI |
| Execution | Shared budget, unique action IDs, state transitions, intent callback | Tokio supervisor + durable intent/outcome store |
| Recovery | Cancellation, unknown-outcome and reconciliation states | Crash-tested persistent reconstruction |
| Models | None | Explicit Ollama, LM Studio, llama.cpp adapters |
| Desktop | None | Rust-rendered eframe/egui UI |
| Tools | None | Constrained read-only workspace tools, staged writes, versioned MCP |
| Coordination | None | Bounded workers with shared limits and provenance |

**Local-first** is an engineering target, not a privacy guarantee already established. An inference server on `localhost` may still delegate to the cloud; future integrations must distinguish endpoint location from actual model location.

## Explore the research

- [Supervised execution](docs/SUPERVISION.md): the new core contract, ordering, invariants, and remaining safety work.
- [Architecture](docs/ARCHITECTURE.md): the boundaries among Rust UI, trusted supervisor, providers, tools, and journal.
- [Requirements](docs/REQUIREMENTS.md) and [roadmap](docs/ROADMAP.md): acceptance gates, not aspirational feature checkmarks.
- [Research and evidence](docs/RESEARCH.md): bounded public-product/REA findings with explicit unknowns.
- [Benchmark plan](docs/BENCHMARKS.md): how future claims of quality could be tested.
- [Security scope](SECURITY.md), [contributing](CONTRIBUTING.md), and [source provenance](PROVENANCE.md): how to build and review responsibly.

## Built in the open

James is an original project from **[Vers3Dynamics](https://vers3dynamics.com/)**. Public Korso product research informed independent requirements. No Korso code, assets, UI, or recovered prompts are shipped here. This is not a formally separated clean-room claim; details live in [PROVENANCE.md](PROVENANCE.md).

**License:** [MIT](LICENSE). No downloadable desktop release exists yet.

> A useful assistant should make the human more capable, not make the human less informed.
