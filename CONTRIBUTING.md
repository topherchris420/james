# Contributing

Start with [README.md](README.md), the [architecture](docs/ARCHITECTURE.md), the [roadmap](docs/ROADMAP.md) and [END_USER_READINESS.md](docs/END_USER_READINESS.md). James is a native Rust desktop app with local-model chat, screen questions, voice, a read-only Lab view and reviewed Windows input. All network access goes through `crates/james-loopback`; do not add another HTTP client. Keep changes small enough to review and state which integration, if any, they actually implement.

## Checks

Run what CI runs before you push:

```sh
cargo fmt --all -- --check
cargo test --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

- The library crates (`james-core`, `james-loopback`, `james-models`, `james-lab`) must keep building with Rust 1.85, and the desktop and platform crates with 1.88: `cargo +1.85.0 test -p james-core -p james-loopback -p james-models -p james-lab --all-targets --locked`.
- Code behind `cfg(windows)` or `cfg(target_os = "macos")` can be type-checked from another system: `rustup target add x86_64-pc-windows-msvc aarch64-apple-darwin`, then `cargo clippy --workspace --all-targets --locked --target <target> -- -D warnings`.
- After editing a PowerShell helper in `crates/james-platform/src`, run `pwsh ./scripts/check-powershell.ps1`; it parses each helper and compiles its embedded C#.
- The GUI tests in `crates/james-desktop/src/tests/gui.rs` drive the real interface headlessly through its AccessKit tree with `egui_kittest`. Give new controls an accessible label (a visible label, `labelled_by` or `widget_info`), and add a test that finds them by that label. A control a test cannot find by name is probably invisible to a screen reader too.
- `cargo run -p james-models --example probe -- <lmstudio|ollama|llamacpp|openai> [port] [--model id] [--chat "question"] [--image file.png]` checks a real model server from the command line, and `cargo run -p james-lab --example inspect -- 5173` a real Lab.

First-time workspace builds need Cargo registry access. Add regression tests for authorization, journal failure, recovery, parsing, cancellation or budget behavior affected by your change. Formatting-only changes do not need new tests. When changing packaging, run `scripts/package-linux.sh` (or `scripts/package-macos.sh` on a Mac) and see [INSTALLER.md](docs/INSTALLER.md).

## Claims and evidence

User-facing wording belongs in [GETTING_STARTED.md](docs/GETTING_STARTED.md) as well as the app. A green CI run proves code compiles and its tests pass on that runner; it does not prove a person can use the feature on that system. Record real runs in [VALIDATION.md](docs/VALIDATION.md), and update [PLATFORM_SUPPORT.md](docs/PLATFORM_SUPPORT.md) and the readiness score only from that evidence. Do not claim comparative advantages without results following [BENCHMARKS.md](docs/BENCHMARKS.md).

Models, model servers, the Lab and extension metadata are untrusted inputs. Only the host classifies effects and validates scope. A passing in-memory contract test does not prove OS confinement or durable execution; include adversarial fixtures and failure-path evidence when implementing those boundaries.

## Provenance

Keep reference source, binaries, captured private data, credentials, certificates and model weights out of commits. Contributions to original James files are under the MIT license. Document the provenance and license of any third-party addition in [PROVENANCE.md](PROVENANCE.md).

PR descriptions should explain the concrete behavior change, relevant tests, supported platforms, and remaining limitations.
