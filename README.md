<div align="center">

# JAMES

### A research assistant learning how to act responsibly.

**One researcher. Two environments. Human authority throughout.**

[![CI](https://github.com/topherchris420/james/actions/workflows/ci.yml/badge.svg)](https://github.com/topherchris420/james/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/Rust-stable%20GUI-0B5D63)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-0B5D63)](LICENSE)
[![Stage](https://img.shields.io/badge/stage-desktop%20preview-555555)](docs/DESKTOP.md)

[**Meet James in R.A.I.N. Lab**](https://github.com/topherchris420/lop-nur-twin#a-lab-behind-a-door) ·
[**Get the Windows preview**](https://github.com/topherchris420/james/actions/workflows/desktop.yml) ·
[**Explore the Rust core**](crates/james-core) ·
[**Read the roadmap**](docs/ROADMAP.md)

</div>

---

> **James should be able to leave the laboratory without leaving its standards behind.**

James is a local-first desktop research companion built around a simple conviction: **intelligence may propose, evidence must be checked, and authority stays with the human.** The project now has a **real native Rust window**, explicit LM Studio model discovery, non-streaming local chat, and local conversation history, alongside separate executable Rust contracts for reviewed actions, bounded runs and fail-closed recovery.

This is a working **text-chat preview**, not a finished agent or signed installer. His desktop environment cannot yet retrieve R.A.I.N. Lab research or execute tools.

## What this repository is

James has an existing home in [R.A.I.N. Lab](https://github.com/topherchris420/lop-nur-twin), where he serves as the lead AI research assistant alongside **Jasmine** (engineering feasibility), **Luca** (geometric and cross-domain connections), and **Elena** (formal and mathematical criticism). That environment owns the live research runtime, evidence corpus, experiments, and team perspectives.

This repository gives James a second environment: a native Rust chat window that talks to an explicitly configured localhost model server. It also develops an **authorization and run-supervision core** for future tool use. The model-facing UI and policy core are separate today; chat does not grant real-world execution authority.

### The honest boundary

The **desktop UI and localhost inference client now exist**, and messages are stored on disk in a plaintext local session file. There is still **no live link to R.A.I.N. Lab**, shared research memory, cross-repository identity protocol, trusted tool executor, screen capture, voice, or durable action journal. The two core examples still simulate approvals and recovery; they do not touch files or execute desktop actions.

The desktop sends messages to the configured loopback endpoint only. This does not establish where the model backend performs inference, and the resulting answers are not automatically scientific evidence.

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
             EXISTS TODAY IN LAB                TEXT CHAT + CORE IN THIS REPO
                    │                                   │
                    └──────────── future, explicit ────────────┘
                              research/context bridge
                         (not implemented or connected)
```

| Environment | Today | Possible future |
| --- | --- | --- |
| **[R.A.I.N. Lab](https://github.com/topherchris420/lop-nur-twin)** | James's research environment, team, experiments, evidence, and native runtime. | The authoritative source of research context and provenance. |
| **James Desktop** | A native Rust text-chat UI, localhost LM Studio adapter, local history, and separately tested authorization/supervision contracts. | Connected research evidence, streamed chat, accessible reviews, policy-gated tools, and crash-tested outcomes. |
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

**The interactive milestone:** [`james-desktop`](crates/james-desktop) opens a native window and talks to a locally configured LM Studio-compatible service through [`james-models`](crates/james-models). It discovers loaded model IDs, sends text prompts, displays responses, and persists conversation history. No third-party installer code or assets are used. The desktop is **not** connected to the Lab's research files.

**The action milestone:** `james-core` separately demonstrates four executable contracts:

1. **Consent is specific.** Approval binds to one action instance, exact invocation, policy snapshot, and expiry. Change the action and approval must be reconsidered.
2. **Budgets belong to the run.** A supervisor owns the quota and uniquely identifies proposed actions. Agents cannot grant themselves more work.
3. **Intent precedes effect.** Authorization and quota are checked before the intent-recording callback. If recording fails, dispatch does not proceed.
4. **Uncertainty has a state.** Interrupted side effects are not automatically retried or labeled successful. They require reconciliation.

These are in-memory contracts—not a completed security boundary. Real durability, user identity, path confinement, resource enforcement, crash recovery, and trusted execution remain future work. A callback that reports persistence is not proof of persistence.

## Launch James

**Windows portable developer build:** Open the [James desktop workflow](https://github.com/topherchris420/james/actions/workflows/desktop.yml), choose a successful run on `main`, download its `James-Windows-x64` artifact, extract it, and double-click `james.exe`. GitHub may require sign-in for artifact downloads. It is unsigned and not a Windows installer.

Start **LM Studio**, load a model, enable its Local Server (usually `127.0.0.1:1234`), then click **Discover models** inside James. Choose a model and start chatting. James does **not** bundle LM Studio or model weights.

**Build from source:** Install Rust stable and platform-native build tools:

```bash
git clone https://github.com/topherchris420/james.git
cd james
cargo fmt --all -- --check
cargo test -p james-core -p james-models -p james-desktop --locked
cargo clippy -p james-core -p james-models -p james-desktop --locked -- -D warnings
cargo run -p james-desktop --locked
```

The core's Rust 1.85 minimum remains supported; the dependency-backed graphical build uses current stable Rust. The old two core demonstrations can still be run with `cargo run -p james-core --example approval_demo` and `cargo run -p james-core --example supervised_demo`. They perform no external actions. First builds fetch crates from the configured Cargo registry.

**Important privacy boundary:** conversation history is saved unencrypted in a local JSON file, and conversation text goes to the chosen localhost model endpoint. Verify the server actually runs your model on-device. Nothing is automatically sent to R.A.I.N. Lab. See [installation and limitations](docs/DESKTOP.md) and [verified checks](docs/VALIDATION.md).

## Where the project is going

| Capability | Implemented now | Planned next |
| --- | --- | --- |
| Authorization | Immutable invocations, one-use approvals, policy revision checks | Keyboard-accessible review UI |
| Execution | Shared budget, action IDs, state transitions, intent callback | Tokio supervisor and durable intent/outcome store |
| Recovery | Cancellation, unknown-outcome, and reconciliation states | Crash-tested persistent reconstruction |
| Models | Localhost LM Studio/OpenAI-compatible non-streaming text chat | Streaming, cancellation, provider-specific capability verification |
| Desktop | Native Rust `eframe`/`egui` window and local history | Accessibility, signing, packaged installers, recovery |
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

- [Desktop quick start](docs/DESKTOP.md) — Windows preview, LM Studio setup, local storage, and known limits.
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

**License:** [MIT](LICENSE). An unsigned Windows executable is provided as a temporary GitHub Actions artifact; there is no signed release or bundled model.

<div align="center">

**A careful beginning is still a beginning.**

</div>
