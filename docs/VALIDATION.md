# Phase 1 validation

Date: 2026-10-11. Scope: `james-core`, its tests, demonstration, and repository preparation. No desktop or comparative benchmark was run.

## Tested configuration

- Host: Windows ARM; Rust test binaries ran using the Windows x64 environment.
- Compiler: rustc 1.99.0, commit `b940084d7eb6a299eb4bfeb8e34901bc051e7ac4`, host target `x86_64-pc-windows-gnu`, LLVM 23.1.1.
- Existing GNU Rust toolchain and w64devkit linker; `RUSTFLAGS=-C link-self-contained=yes`.
- `james-core` has no third-party dependencies; locked Cargo operations ran offline.

The initial host-native MSVC attempt could not find `link.exe`. A GNU attempt without the self-contained option could not locate `libgcc_eh`. The final configured GNU run passed. These environment failures are not reported as passed attempts. The missing GNU Clippy component was added through rustup before the final strict check.

## Completed checks

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed |
| `cargo test --workspace --all-targets --locked --offline` | 26 integration regression tests passed; zero failures |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | Passed |
| `cargo run --locked --offline --example approval_demo` | Passed; simulated review → running → needs reconciliation; replay rejected; explicit reconciliation completed |

The example performs no real operation. Its simulated approval is not an actual user authorization.

## What the tests establish

Regression tests exercise exact invocation matching, separate-action approval transfer rejection, outstanding-token invalidation after recovery, policy changes including a missed revision increment, expiry at review and dispatch, denied tools, illegal transitions, single-action replay rejection, run-bound budget accounting, token overflow, deadlines, pre-dispatch cancellation, cancellation intent versus confirmed result, and interrupted-operation reconciliation.

Approvals are bound to one in-memory action generation. Budgets are non-copyable and run-bound, but the trusted supervisor must still own exactly one budget and a durable registry of unique actions. Constructing new objects is not globally prevented by this crate. Time is supplied by the trusted host using one monotonic clock.

## Reproduce the Windows check

From the repository, with an existing compatible GNU installation:

```powershell
.\scripts\check.ps1 -RustBin 'C:\Users\chris\.rustup\toolchains\stable-x86_64-pc-windows-gnu\bin' -LinkerDirectory 'C:\Users\chris\.local\triad-toolchain\w64devkit\bin'
```

The paths describe this tested machine, not required directory names for other users. A standard configured Rust installation can run the README commands directly. The script sets environment variables only in its current shell process; it does not alter persistent system configuration.

## Not established

No real screen capture, inference, UI, MCP, filesystem confinement, durable crash recovery, extension isolation, credential storage, agent coordination, local-only network guarantee, or relative performance was tested. Native Windows ARM/MSVC, macOS, Linux, and the Rust 1.85 minimum remain untested locally. The CI workflow is prepared for stable and 1.85.0 across three OS runners; it has not executed remotely.
