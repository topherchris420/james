<div align="center">

# JAMES

### A research assistant learning how to act responsibly.

**One researcher. Two environments. Human authority throughout.**

[![CI](https://github.com/topherchris420/james/actions/workflows/ci.yml/badge.svg)](https://github.com/topherchris420/james/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/Rust-1.85%2B-0B5D63)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-0B5D63)](LICENSE)
[![Stage](https://img.shields.io/badge/stage-core%20prototype-555555)](docs/ROADMAP.md)

[**Meet James in R.A.I.N. Lab**](https://github.com/topherchris420/lop-nur-twin#a-lab-behind-a-door) ·
[**Explore the Rust core**](crates/james-core) ·
[**Read the roadmap**](docs/ROADMAP.md)

</div>

---

> **James should be able to leave the laboratory without leaving its standards behind.**

James is the beginning of a local-first desktop assistant built around a simple conviction: **intelligence may propose, evidence must be checked, and authority stays with the human.** This repository contains the first executable Rust contracts for that idea—reviewed actions, bounded runs, fail-closed recovery, and supervised intent recording.

It is not a finished desktop product. It is a deliberately small foundation for one.

## What this repository is

James has an existing home in [R.A.I.N. Lab](https://github.com/topherchris420/lop-nur-twin), where he serves as the lead AI research assistant alongside **Jasmine** (engineering feasibility), **Luca** (geometric and cross-domain connections), and **Elena** (formal and mathematical criticism). That environment owns the live research runtime, evidence corpus, experiments, and team perspectives.

This repository explores James in a second environment: a Rust desktop host where he could eventually explain work, ask for consent, interact with explicitly selected resources, and show what happened. Today, it is an independently executable **authorization and run-supervision prototype**.

### The honest boundary

There is currently **no** shared runtime, desktop UI, inference backend, persistent memory, research synchronization, cross-repository identity protocol, trusted tool executor, or durable action journal here. The examples do not touch your files, control your desktop, call a model, or perform a durable write.

That boundary is a feature. The project documents what exists, what is proposed, and what still has to be proven before the system earns more authority.

## One James, two environments

```text
                         JAMES · LEAD RESEARCH ASSISTANT
                                      │
                    ┌─────────────────┴─────────────────┐
                    │                                   │
                R.A.I.N. LAB                        JAMES DESKTOP
              Research environment                  Rust environment
                    │                                   │
        Papers · hypotheses · experiments      Review · budgets · action state
        Evidence · replay · manuscripts        User-controlled execution
                    │                                   │
             EXISTS TODAY IN LAB                CORE PROTOTYPE IN THIS REPO
                    │                                   │
                    └──────────── future, explicit ────────────┘
                              research/context bridge
                         (not implemented or connected)
```

| Environment | Today | Possible future |
| --- | --- | --- |
| **[R.A.I.N. Lab](https://github.com/topherchris420/lop-nur-twin)** | James's research environment, team, experiments, evidence, and native runtime. | The authoritative source of research context and provenance. |
| **James Desktop** | Dependency-free Rust contracts for action review, permissions, budgets, state transitions, and in-memory run supervision. | A human-directed desktop with transparent conversations, approvals, tools, and outcomes. |
| **Bridge** | Not implemented. No live integration or automatic memory transfer exists. | A versioned, read-only-first handoff for evidence-linked summaries and user-approved requests. |

The bridge must never become a shortcut around either environment's rules. A research claim is not automatically proof. A proposal is not permission. Consent in one environment must not silently become authority in the other.

## The operating principle

> **Intelligence may propose. Evidence must be checked. Authority stays with the human.**

```text
      QUESTION OR RESEARCH FINDING
                    │
                    ▼
             James proposes
                    │
          Immutable action intent
                    │
             Human reviews
                    │
          Policy + budget gate
                    │
          Record exact intent
                    │
           Trusted executor
                    │
         Verify observed outcome
                    │
                 Timeline
```

The path above is the intended host design, not a claim about the current implementation. This repository currently contains the in-memory policy, action, budget, supervisor, and simulated intent-callback portions. It does not yet contain a trusted executor or durable journal.

## What is implemented now

The `james-core` crate currently demonstrates four executable contracts:

1. **Consent is specific.** Approval binds to one action instance, exact invocation, policy snapshot, and expiry. Change the action and approval must be reconsidered.
2. **Budgets belong to the run.** A supervisor owns the quota and uniquely identifies proposed actions. Agents cannot grant themselves more work.
3. **Intent precedes effect.** Authorization and quota are checked before the intent-recording callback. If recording fails, dispatch does not proceed.
4. **Uncertainty has a state.** Interrupted side effects are not automatically retried or labeled successful. They require reconciliation.

These are in-memory contracts—not a completed security boundary. Real durability, user identity, path confinement, resource enforcement, crash recovery, and trusted execution remain future work. A callback that reports persistence is not proof of persistence.

## Run the prototype

Requires **Rust 1.85+** and a configured linker. `james-core` has no third-party Rust dependencies.

```bash
git clone https://github.com/topherchris420/james.git
cd james

# Format, test, and lint the workspace
cargo fmt --all -- --check
cargo test --workspace --all-targets --locked --offline
cargo clippy --workspace --all-targets --locked --offline -- -D warnings

# Walk through the two executable demonstrations
cargo run --locked --offline --example approval_demo
cargo run --locked --offline --example supervised_demo
```

The approval demo shows an interrupted action that cannot be replayed. The supervisor demo shows distinct action identity, an in-memory intent callback, and a secret-free execution timeline. **Neither demo performs an actual tool call or durable write.** On Windows, [`scripts/check.ps1`](scripts/check.ps1) supports local Rust/linker overrides.

For measured results and platform scope, see [validation](docs/VALIDATION.md).

## Where the project is going

| Capability | Implemented now | Planned next |
| --- | --- | --- |
| Authorization | Immutable invocations, one-use approvals, policy revision checks | Keyboard-accessible review UI |
| Execution | Shared budget, action IDs, state transitions, intent callback | Tokio supervisor and durable intent/outcome store |
| Recovery | Cancellation, unknown-outcome, and reconciliation states | Crash-tested persistent reconstruction |
| Models | None | Explicit Ollama, LM Studio, and llama.cpp adapters |
| Desktop | None | Rust-rendered `eframe`/`egui` interface |
| Tools | None | Constrained read-only tools, staged writes, versioned MCP |
| R.A.I.N. integration | None; the Lab remains a separate repository | Evidence-linked, versioned research handoff |
| Coordination | None in the desktop Rust host | Bounded workers with shared budgets and provenance |

The full roadmap is staged from executable core contracts to a working desktop, controlled tools, provider compatibility, bounded coordination, desktop context, and finally a release that can publish only measured claims. See [docs/ROADMAP.md](docs/ROADMAP.md).

## Design commitments

- **Human review is concrete.** Approvals refer to exact invocations, not vague intentions.
- **Fail closed at the boundary.** Policy, budget, cancellation, and revocation are checked before side effects.
- **Unknown means unknown.** A crash after an external effect and before its result is saved is not safe to retry automatically.
- **Local-first is a target, not a guarantee.** A localhost endpoint can still delegate to the cloud; endpoint location and inference location must be distinguished.
- **Extensions are not automatically sandboxes.** MCP metadata, processes, credentials, and outputs require explicit boundaries and visible trust labels.
- **Claims require evidence.** Tests establish contracts; they do not prove integrations, isolation, accessibility, or product superiority.

## Read the project as a system

- [Architecture](docs/ARCHITECTURE.md) — boundaries, proposed integrations, and design decisions.
- [Supervised execution](docs/SUPERVISION.md) — ordering, invariants, and remaining safety work.
- [Requirements](docs/REQUIREMENTS.md) — acceptance gates rather than aspirational checklists.
- [Validation](docs/VALIDATION.md) — what has actually been exercised and measured.
- [Research and evidence](docs/RESEARCH.md) — bounded findings with explicit unknowns.
- [Benchmark plan](docs/BENCHMARKS.md) — how future quality claims should be tested.
- [Security scope](SECURITY.md) — reporting and security boundaries.
- [Provenance](PROVENANCE.md) — the origin and boundaries of this implementation.
- [Contributing](CONTRIBUTING.md) — how to participate responsibly.

## Built in the open

James is part of **[Vers3Dynamics](https://vers3dynamics.com/)**. This Rust implementation is original work; public product research informed its independently designed desktop requirements. No third-party reference code, assets, interface designs, or recovered prompts are shipped here. This is not a formally separated clean-room claim; the details live in [PROVENANCE.md](PROVENANCE.md).

**License:** [MIT](LICENSE). No downloadable desktop release exists yet.

<div align="center">

**A careful beginning is still a beginning.**

</div>
