# James

An original, open-source Rust desktop AI assistant in development.

**Current milestone: Phase 1 — research, architecture, and executable core contracts.** James does not yet have a desktop interface, model integration, screen capture, voice, tools, persistence, or an agent runtime. The example simulates authorization and recovery; it performs no desktop action.

## What exists

- A Korso research ledger with REA Evidence IDs, artifact hashes, and explicit coverage limits.
- An independent architecture for a Rust-rendered desktop, local-model adapters, supervised extensions, durable recovery, and bounded agent coordination.
- A dependency-free Rust core for immutable action review, one-use approvals, permission changes, run budgets, cancellation, and interrupted-operation reconciliation.
- Regression tests, a contract demonstration, and a proposed comparison protocol.

## Run the contracts

Use a configured Rust 1.85 or newer toolchain and the linker required by its target. The crate has no third-party dependencies.

```sh
cargo fmt --all -- --check
cargo test --workspace --all-targets --locked --offline
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
cargo run --locked --offline --example approval_demo
```

On Windows, `scripts/check.ps1` runs those checks. It accepts optional `-RustBin` and `-LinkerDirectory` arguments for an existing GNU toolchain. See [validation](docs/VALIDATION.md) for the exact tested configuration and limitations. The Rust 1.85 minimum is a manifest target; only the compiler recorded there has been tested locally.

## Design direction

The proposed desktop uses eframe/egui, with a Tokio supervisor and local SQLite journal. Model adapters initially target Ollama, LM Studio, and llama.cpp servers. Provider locality, capabilities, and data destinations will be visible; a loopback endpoint alone does not prove on-device inference. Extensions will use explicit capabilities and versioned MCP profiles. Arbitrary installed extensions remain trusted code until OS sandboxing is implemented and tested.

James's original interface will center on conversations, explicit context selection, concrete action review, and an execution timeline. Model proposals cannot grant themselves authority. An interrupted external action has an unknown outcome until reconciled.

## Project documents

- [Research and evidence](docs/RESEARCH.md)
- [Architecture decisions](docs/ARCHITECTURE.md)
- [Requirements and acceptance criteria](docs/REQUIREMENTS.md)
- [Implementation roadmap](docs/ROADMAP.md)
- [Benchmark protocol](docs/BENCHMARKS.md)
- [Validation results](docs/VALIDATION.md)
- [Source provenance](PROVENANCE.md)
- [Contribution guide](CONTRIBUTING.md)
- [Security scope](SECURITY.md)
- [Publication checklist](docs/PUBLISHING.md)

Korso is a research reference, not a dependency. James contains no Korso code, assets, prompts, or interfaces. No comparative benchmark has been run, and no superiority claim is made. The Phase 1 source repository is [topherchris420/james](https://github.com/topherchris420/james); there is no usable desktop release yet.

Licensed under [MIT](LICENSE). The license applies to James's original files; reference artifacts remain outside this repository.
