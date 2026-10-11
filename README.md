<div align="center">

# JAMES

### One researcher. Two environments. Human authority throughout.

**The R.A.I.N. Lab's lead assistant, growing toward a local-first desktop presence in Rust.**

[![CI](https://github.com/topherchris420/james/actions/workflows/ci.yml/badge.svg)](https://github.com/topherchris420/james/actions/workflows/ci.yml)
![Rust](https://img.shields.io/badge/Rust-1.85%2B-0B5D63)
![MIT](https://img.shields.io/badge/license-MIT-0B5D63)
![Prototype](https://img.shields.io/badge/stage-core%20prototype-555555)

[Meet James in R.A.I.N. Lab](https://github.com/topherchris420/lop-nur-twin#a-lab-behind-a-door) ·
[Explore the Rust core](crates/james-core) ·
[See the roadmap](docs/ROADMAP.md)

</div>

---

**James already has a laboratory. This is the beginning of a doorway out of it.**

Inside [R.A.I.N. Lab](https://github.com/topherchris420/lop-nur-twin), James is the lead AI research assistant: the evidence-focused voice in a team with **Jasmine** (engineering feasibility), **Luca** (geometric and cross-domain connections), and **Elena** (formal and mathematical criticism). Together, they investigate questions, challenge assumptions, and distinguish hypotheses from simulator evidence.

This repository is **that same James**, not an unrelated assistant with the same name. It explores a second environment for him: a Rust desktop interface through which, eventually, you could ask about laboratory findings, review proposed work, and authorize carefully scoped actions on your computer.

**One James is the product direction, not yet a shared running implementation.** The working R.A.I.N. research runtime lives in `lop-nur-twin`; this repository currently contains an independently executable **Rust authorization and run-supervision prototype**. There is **no connection, shared memory, research synchronization, desktop UI, inference backend, or cross-repository identity protocol implemented here yet**. None of the examples touches your files or controls a desktop.

## One James, two environments

```text
                  JAMES · LEAD RESEARCH ASSISTANT
                               (shared identity)
                                       │
                     ┌─────────────────┴─────────────────┐
                     │                                   │
                 R.A.I.N. LAB                         JAMES DESKTOP
             Research environment                     Rust environment
                     │                                   │
          James · Jasmine · Luca · Elena          Review · budget · action state
          Papers · hypotheses · experiments       User-controlled execution
          Evidence · replay · manuscripts         Local models · desktop UX
                     │                                   │
              EXISTS IN R.A.I.N.                    CORE PROTOTYPE ONLY
                     │                                   │
                     └───────── future, explicit ────────┘
                           research/context bridge
                      (NOT IMPLEMENTED OR CONNECTED)
```

| Environment | What it is today | What the relationship could become |
| --- | --- | --- |
| **[R.A.I.N. Lab](https://github.com/topherchris420/lop-nur-twin)** | An integrated research environment in the Bethesda simulation. Its native runtime, evidence corpus, experiments and team perspectives live in [`src/rain/`](https://github.com/topherchris420/lop-nur-twin/tree/main/src/rain) and [`src/bethesda/rain/`](https://github.com/topherchris420/lop-nur-twin/tree/main/src/bethesda/rain). | The authoritative source of James's research context, experiment provenance, and collaborations. |
| **James Desktop (this repository)** | Dependency-free Rust contracts for immutable action review, permissions, budgets, state transitions and in-memory run supervision. | A human-directed desktop where James can explain work, request consent, interact with explicitly selected resources, and show observable outcomes. |
| **Bridge between them** | **Not implemented.** No live integration, common conversation store or automatic identity/memory transfer exists. | A versioned, read-only-first interface for evidence-linked research summaries and user-approved requests, with separate execution authority and auditable provenance. |

The bridge must not be a shortcut around either environment's rules. A claim from the Lab is not automatically scientific proof; a proposal from James is not permission to act; and consent granted in one environment must not silently become a grant in the other.

## The rule James carries with him

> **Intelligence may propose. Evidence must be checked. Authority stays with the human.**

```text
           QUESTION OR RESEARCH FINDING
                         │
                         ▼
                 James proposes
                         │
                 Immutable action
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
                      TIMELINE
```

The diagram describes the **intended end-to-end host design**. Today, only the in-memory policy, action, budget, supervisor and simulated intent-callback portions exist in this Rust repository. There is no trusted tool executor or durable action journal yet.

## Four executable core contracts

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

## The desktop implementation: built versus planned

| Capability | Implemented now | Planned |
| --- | --- | --- |
| Authorization | Immutable invocations, one-use approvals, policy revision checks | Actual keyboard-accessible review UI |
| Execution | Shared budget, unique action IDs, state transitions, intent callback | Tokio supervisor + durable intent/outcome store |
| Recovery | Cancellation, unknown-outcome and reconciliation states | Crash-tested persistent reconstruction |
| Models | None | Explicit Ollama, LM Studio, llama.cpp adapters |
| Desktop | None | Rust-rendered eframe/egui UI |
| Tools | None | Constrained read-only workspace tools, staged writes, versioned MCP |
| R.A.I.N. integration | None; Lab remains in another repository | Evidence-linked, versioned research handoff; no automatic authority transfer |
| Coordination | None in desktop Rust | Bounded workers with shared limits and provenance |

**Local-first** is an engineering target, not a privacy guarantee already established. An inference server on `localhost` may still delegate to the cloud; future integrations must distinguish endpoint location from actual model location.

## Follow the research

- [R.A.I.N. Lab](https://github.com/topherchris420/lop-nur-twin): James's research environment, team, experiments and evidence.
- [Lab migration record](https://github.com/topherchris420/lop-nur-twin/blob/main/docs/RAIN_MIGRATION.md): why the live research runtime now lives inside `lop-nur-twin`.
- [Supervised execution](docs/SUPERVISION.md): Rust core ordering, invariants, and remaining safety work.
- [Architecture](docs/ARCHITECTURE.md): the boundaries among Rust UI, trusted supervisor, providers, tools, and journal.
- [Requirements](docs/REQUIREMENTS.md) and [roadmap](docs/ROADMAP.md): acceptance gates, not aspirational feature checkmarks.
- [Research and evidence](docs/RESEARCH.md): bounded public-product/REA findings with explicit unknowns.
- [Benchmark plan](docs/BENCHMARKS.md): how future claims of quality could be tested.
- [Security scope](SECURITY.md), [contributing](CONTRIBUTING.md), and [source provenance](PROVENANCE.md): how to build and review responsibly.

## Built in the open

James is part of **[Vers3Dynamics](https://vers3dynamics.com/)** and shares its name and intended assistant identity with the lead researcher in R.A.I.N. Lab. This Rust implementation is original work; publicly available product research informed its independently designed desktop requirements. No third-party reference code, assets, interface designs, or recovered prompts are shipped here. This is not a formally separated clean-room claim; details live in [PROVENANCE.md](PROVENANCE.md).

**License:** [MIT](LICENSE). No downloadable desktop release exists yet.

> James should be able to leave the laboratory without leaving its standards behind.
