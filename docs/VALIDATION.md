# Implementation validation

## v0.3 integration pass (2026-10-11)

The records below this section describe earlier milestones. Current v0.3 adds native screen and voice helpers, reviewed Windows input, the real Lab discovery client, and installer packaging. Protocol and contract checks do not establish inference quality or automatic outcome verification.

- Actual Lab Vite server: read-only status and discovery verified at upstream commit `a2d7882e1317af1ea1a333c31ead033f0d87c60e`; reachable, available, idle. No experiment started.
- Windows helpers: in-memory PNG capture and cancellation of launched speech helpers exercised. A capture test hit the original 20-second deadline under host load; subsequent compiled-test and direct-helper retries passed in 12.30 and 11.41 seconds, returning a 1920×1200 PNG. After increasing the cancellable startup deadline to 60 seconds, all seven platform tests (including both opt-in smoke tests) passed in 7.66 seconds. Microphone accuracy and actual target-application input remain separate hands-on checks.
- REA public Korso page observation: bounded finding ledger in [DESKTOP-RESEARCH.md](DESKTOP-RESEARCH.md). No desktop-internals equivalence claimed.
- Inno Setup 6.4.3 template compilation and script parameter/path validation passed. The portable compiler download passed package SHA-256 and valid publisher-signature verification. Clean-machine installation lifecycle and signing remain outstanding.
- [Desktop CI run 38153765315](https://github.com/topherchris420/james/actions/runs/38153765315) subsequently passed Windows and Ubuntu tests/lint, built the optimized Windows executable, compiled the real installer, and uploaded both distributions with the installer checksum. This is build verification, not a clean-machine installation test. The later capture-timeout-only adjustment also passed all seven local platform tests.
- At commit `1433a22dac97f102eda18bd321f62a5949038cd1`, [desktop CI](https://github.com/topherchris420/james/actions/runs/38152527843) passed all 56 workspace tests, formatting and strict Clippy on Windows and Ubuntu. Windows also completed the optimized MSVC executable build. Packaging then failed because Chocolatey did not provide the pinned compiler; the portable verified compiler replaces that dependency. The separate six-job core Rust compatibility matrix also passed.
- The local Rust 1.99.0 GNU workspace check and debug desktop build succeeded. The native window was launched and its Settings, Screen, Computer control and Lab panels inspected. Attempting to review a key without a capture was correctly blocked. This was a navigation and guard check, not an executed input action or usability study.
- No LM Studio server was available on local port 1234. Model/vision protocol tests passed using mock HTTP servers; real inference, screenshot interpretation and proposal quality were not exercised. No microphone accuracy claim is made.
- Inspection of the first optimized executable found `VCRUNTIME140.dll` among its imports. The final Windows distribution configuration uses static CRT linkage and CI now rejects a separate VC runtime dependency before packaging. Earlier preview artifacts predate this packaging correction.

### Earlier milestone history

Date: 2026-10-11. Scope: `james-core`, the new LM Studio adapter, the desktop build and CI. No human desktop usability study or comparative benchmark was run.

## Original local tested configuration

- Host: Windows ARM; Rust test binaries ran using the Windows x64 environment.
- Compiler: rustc 1.99.0, commit `b940084d7eb6a299eb4bfeb8e34901bc051e7ac4`, host target `x86_64-pc-windows-gnu`, LLVM 23.1.1.
- Existing GNU Rust toolchain and w64devkit linker; `RUSTFLAGS=-C link-self-contained=yes`.
- `james-core` has no third-party dependencies; locked Cargo operations ran offline.

The initial host-native MSVC attempt could not find `link.exe`. A GNU attempt without the self-contained option could not locate `libgcc_eh`. The final configured GNU run passed. These environment failures are not reported as passed attempts. The missing GNU Clippy component was added through rustup before the final strict check.

## Original local checks (initial commit)

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed |
| `cargo test --workspace --all-targets --locked --offline` | 26 integration regression tests passed; zero failures |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | Passed |
| `cargo run --locked --offline --example approval_demo` | Passed; simulated review → running → needs reconciliation; replay rejected; explicit reconciliation completed |

The example performs no real operation. Its simulated approval is not an actual user authorization.

## What the tests establish

Regression tests exercise exact invocation matching, separate-action approval transfer rejection, outstanding-token invalidation after recovery, policy changes including a missed revision increment, expiry at review and dispatch, denied tools, illegal transitions, single-action replay rejection, run-bound budget accounting, token overflow, deadlines, pre-dispatch cancellation, cancellation intent versus confirmed result, and interrupted-operation reconciliation.

Approvals are bound to one in-memory action generation. Budgets are non-copyable and run-bound. The new in-memory `RunSupervisor` owns one budget and generates unique ephemeral IDs, but a real host still needs a durable registry and recovery implementation. Constructing new objects is not globally prevented by this crate. Time is supplied by the trusted host using one monotonic clock.

## Hosted CI validation after supervision upgrade

[GitHub Actions run 38147117780](https://github.com/topherchris420/james/actions/runs/38147117780) completed successfully on 2026-10-11.

| Runner | Rust 1.85.0 | Rust stable |
| --- | --- | --- |
| Ubuntu | Passed | Passed |
| Windows | Passed | Passed |
| macOS | Passed | Passed |

Each matrix job ran locked/offline tests, strict Clippy, and the original no-I/O approval demonstration. Stable jobs additionally ran rustfmt verification. The enhanced source includes the original 26 regression tests and **7 additional supervisor regression tests**; the second demonstration was added and compiled as an example target.

New test scenarios cover action ID uniqueness, one shared budget, failed intent recording, policy/consent expiry, no journal callback for unauthorized dispatch, interrupted effects requiring reconciliation, cancellation, and token usage limits.

The source's first supervised-execution CI run failed **formatting only**; the formatting-only fix then passed all six combinations. This is a transparent development history, not a performance benchmark. The in-memory recorder remains a mock, not durable storage.

## Reproduce the Windows check

From the repository, with an existing compatible GNU installation:

```powershell
.\scripts\check.ps1 -RustBin 'C:\Users\chris\.rustup\toolchains\stable-x86_64-pc-windows-gnu\bin' -LinkerDirectory 'C:\Users\chris\.local\triad-toolchain\w64devkit\bin'
```

The paths describe this tested machine, not required directory names for other users. A standard configured Rust installation can run the README commands directly. The script sets environment variables only in its current shell process; it does not alter persistent system configuration.

## Not established

No real screen capture, inference, UI, MCP, filesystem confinement, durable crash recovery, extension isolation, credential storage, agent coordination, local-only network guarantee, or relative performance was tested. Native Windows ARM/MSVC, macOS, Linux, and Rust 1.85 remain untested on the original local host. Hosted CI has now passed on Windows, macOS, and Ubuntu using both stable and Rust 1.85.0; hosted CI is not equivalent to hands-on desktop-runtime validation.


## Desktop preview validation (2026-10-11)

- **Core:** [six-configuration workflow run 38148904460](https://github.com/topherchris420/james/actions/runs/38148904460) passed the dependency-aware `james-core` tests and strict Clippy on Ubuntu, Windows and macOS, using Rust stable and 1.85.0.
- **Desktop:** [workflow run 38148904458](https://github.com/topherchris420/james/actions/runs/38148904458) passed formatting, `james-models` and `james-desktop` compilation/tests, and strict Clippy on Ubuntu and Windows. The Windows job built the optimized executable and uploaded the `James-Windows-x64` artifact (approximately 2.4 MB compressed).
- **Six provider unit tests:** deterministic model-list parsing; chat parsing and malformed-result rejection; local HTTP model discovery; local HTTP text completion; non-success HTTP rejection; invalid configuration and message-size rejection. The mock server runs on random loopback ports only.
- **Not yet verified:** a human-operated Windows GUI session using an actual LM Studio/Qwen model; voice, streaming and cancellation; signed installers; OS sandboxing; accessibility; Lab integration; durable action execution. Compilation and mock HTTP tests do not establish these outcomes.

The addition of third-party GUI and HTTP dependencies means a fresh checkout needs Cargo registry access even when only testing the dependency-free `james-core` package within the shared workspace. The original offline test record describes the pre-desktop workspace only. The current CI runs core tests with `--locked` and online dependency resolution, while desktop builds use stable Rust and the checked-in lockfile.
