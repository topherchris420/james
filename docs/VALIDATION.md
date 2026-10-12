# Implementation validation

This ledger records what was actually checked, where, and what was found,
newest first. [END_USER_READINESS.md](END_USER_READINESS.md) turns it into a
score and release classification; [ACCEPTANCE_TESTS.md](ACCEPTANCE_TESTS.md)
lists the scenarios still to run.

## End-user release session (2026-10-12)

James 0.4.0: guided setup, model capabilities, cloud-model warnings,
Markdown, themes and accessibility settings, privacy controls, a richer
action record, a Windows target-window check, R.A.I.N. Lab summaries, and
release packaging. Everything below ran on one 4-vCPU Linux x64 cloud VM
(Ubuntu 24.04.5, no GPU, no audio) with Rust 1.97.0. **No Windows or macOS
machine was used by a person; those platforms have CI evidence only.**

### Automated checks

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed |
| `cargo test --workspace --all-targets --locked` | 192 passed, 0 failed, 2 ignored on Linux (opt-in screen-capture tests; the Windows helper tests compile only on Windows) |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` for `x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc` and `aarch64-apple-darwin` | Passed on all three (Windows and macOS type-checked from Linux) |
| Rust 1.85.0: tests and Clippy for `james-core`, `james-loopback`, `james-models`, `james-lab` | Passed after removing one let-chain that 1.85 rejects |
| Rust 1.88.0: `cargo check --all-targets` of `james-desktop` and `james-platform` for Linux, Windows and macOS targets | Passed. Rust 1.85 rejects their let-chains (E0658), which the README's "1.85+" badge had hidden; both crates now declare `rust-version = "1.88"` and CI checks it |
| `scripts/check-powershell.ps1` (PowerShell 7 on Linux) | All four helpers parse; their C# compiles. A planted syntax error and a broken C# struct were both caught |
| `appstreamcli validate`, `desktop-file-validate`, `lintian` on the `.deb` | Passed (lintian with four documented suppressions) |
| Third-party notices (`tools/notices`) | Linux x64: 259 crates, 162 distinct license texts; Windows x64: 178 crates; macOS (both slices): 187 crates. Standard SPDX texts were needed for 25-44 crates per target that publish no license file (egui, AccessKit, objc2 and others); the egui fonts' OFL and Ubuntu Font Licence texts are found in the crate. The `.deb` with the notices passed lintian with no new tags |
| GitHub Actions at `85a6b21`: [James desktop 38223328461](https://github.com/topherchris420/james/actions/runs/38223328461) and [Core Rust contracts 38223328485](https://github.com/topherchris420/james/actions/runs/38223328485) | All jobs passed: tests, Clippy and packaging on Windows, macOS and Ubuntu; the Ubuntu 22.04 package container including `appstreamcli` and `lintian`; Rust 1.85 and stable on all three systems. The previous run (38222147035) failed only on the 22.04 AppStream validator, fixed in `85a6b21` |
| Windows runner real-session tests (same run) | Capture recorded the window in front ("hosted-compute-agent", WindowsTerminal); input aimed at a window that was not in front was refused before anything was sent. Inno Setup 6.4.3 compiled the uninstaller's `[Code]` section. The executable imports no Visual C++ runtime DLL |

New tests: 20 GUI tests drive the real interface headlessly through its
AccessKit tree (setup and resume, streaming and Markdown, Stop, errors and
retry, Ask again, vision gating, cloud warnings, keyboard-only navigation,
shortcuts, delete-all, history off, crash recovery, layout at 760×560 to
1920×1080 at 1x, 1.25x and 2x with text at 100-200% in every theme, the Lab
view, Lab offline back-off, platform honesty, glyph coverage, screen
questions without history), plus two Windows-only GUI tests (Deny, countdown
Cancel) that run on the Windows runner. Model-client tests use response
shapes recorded from the real servers below.

### Real model servers

- **llama.cpp** built from tag `b11564` (commit 08ffba2), CPU, one thread,
  serving SmolVLM2-500M-Video-Instruct Q8_0 with its projector (vision,
  `-c 4096`) and Qwen2.5-1.5B-Instruct q4_k_m (text, `-c 1024`), downloaded
  from Hugging Face (`ggml-org`, `Qwen`).
- **Ollama** built from tag `v0.40.3` (it reports version 0.0.0 because a
  source build carries no release stamp), with `qwen2.5:0.5b`,
  `nomic-embed-text`, `gemma3:4b` and the cloud stub `gpt-oss:20b-cloud`
  pulled from the Ollama registry. No Ollama account was signed in.
- LM Studio was **not** available; its support remains fixture-tested only.

Through `cargo run -p james-models --example probe`:

| Probe | Result |
| --- | --- |
| llama.cpp SmolVLM capabilities | vision Yes, context 4096 (from `/v1/models`) |
| llama.cpp text chat | streamed 194 bytes; first text 183 ms, done 1.4 s |
| llama.cpp vision (800×400 PNG with a blue "Save changes" button under "Settings: Notifications") | named both labels; first text 13.2 s; ignored "button text only" |
| llama.cpp Qwen2.5-1.5B with an image | HTTP 500 "image input is not supported…" → classified *no images* |
| llama.cpp ~3,200-token prompt at `-c 1024` | HTTP 400 "request (3234 tokens) exceeds the available context size (1024 tokens)" → *context full* |
| Ollama listing | `gpt-oss:20b-cloud` carries `remote_host: https://ollama.com` in `/api/tags`, but **not** in `/api/show`; James now reads both |
| Ollama capabilities | gemma3 vision Yes; nomic-embed chat No; qwen2.5:0.5b vision No; after loading, gemma3 context 4096 from `/api/ps` |
| Ollama gemma3:4b vision, same PNG, "label only" | "Save changes"; first text 37.5 s |
| Ollama errors | embedding model chat → HTTP 400 "does not support chat" (*not a chat model*); text model + image → "Multimodal data provided, but model does not support multimodal requests." (*no images*); unknown model → 404 (*model missing*); cloud model → HTTP 401 "Unauthorized" (*needs sign-in*) |

### Real interface sessions

The app ran under Xvfb (1400×1000 at 1x and 2240×1640 at 2x) with Mesa
software OpenGL and no window manager, driven by `xdotool`.

1. **First-run setup** with Ollama and llama.cpp running: **Find my model
   server** chose Ollama `gemma3:4b`, skipping the cloud and embedding
   models; the facts line read "can read images, loads on first use";
   choosing `gpt-oss:20b-cloud` showed the remote warning. The test message
   answered in 16-30 s including model load. Progress was saved after each
   step.
2. **Bug: typing did not reach the message box.** The new text box's id
   equalled its panel's id ("composer"). Confirmed by typing into an
   unmodified `main` build, fixed, and covered by a GUI test.
3. **Bug: "→" and "│" rendered as empty boxes** in egui's proportional font.
   Replaced; a test now checks every non-ASCII character in the UI source.
4. **Upgrade.** `james_0.3.0_amd64.deb` built from `main` (3ed88d9) was
   installed; 0.3.0 found Ollama and held a real conversation, saved with
   permissions 0644 in a 0755 folder. An action record in the 0.3 format
   was then planted (Linux 0.3 never writes one; Windows 0.3 does).
   `apt install ./james_0.4.0_amd64.deb` unpacked 0.4.0 over 0.3.0. On
   launch, setup was skipped, the model and conversation were kept, the
   folder became 0700 and the session 0600, and the old record was shown as
   "Type “hello from 0.3” (14 characters)" with an unknown outcome until
   acknowledged. (That record itself stayed 0644; fixed so every file James
   owns is tightened.) `apt remove james` removed the program and kept
   `~/.james`; the archive's `install.sh` and `--uninstall` worked.
5. **Markdown** from gemma3 rendered, but list numbers sat off the baseline
   and bold was invisible because the theme gave "strong" text the body
   colour. Both fixed.
6. **Stop.** During a 600-word essay, Esc after about 25 s kept 750 bytes
   labelled "[Stopped by you before the reply finished.]". Ollama's log then
   showed `srv stop: cancel task` and idle slots: **Ollama 0.40.3 stops
   generating when James closes the connection.**
7. **Guide me.** SmolVLM2-500M accepted the screenshot (a 1,087-token
   prompt) but answered by paraphrasing the system prompt. Gemma 3 then
   repeated an earlier answer word for word, which exposed a design flaw:
   Guide me sent the whole conversation. Screen questions now send no
   history; afterwards Gemma 3 named the visible "Conversation" panel, an
   on-screen heading, the message box and the Stop button (about 2.5
   minutes end to end on this CPU).
8. **R.A.I.N. Lab** at lop-nur-twin `4871ef6`, run with Node 22.22
   (`npm ci`, `npm run dev`). **Find the Lab** found port 5173; the page
   showed "lop-nur-twin 4871ef6 · scripted meetings", the idle stage in
   words, no finished experiments, and the knowledge graph's 62 Lab-observed
   and 49 source items. No experiment was started.
9. The README and guide screenshots in `docs/assets` come from this session
   at 2x scale, unedited.

### What this does not establish

- Nothing ran on a physical Windows PC or Mac: installers, SmartScreen,
  Gatekeeper, permission prompts, Start menu and Applications launch remain
  unrecorded. **No reviewed click or keystroke was sent to a real
  application** in this release's testing.
- No physical Linux desktop, GPU driver or Wayland session; no microphone,
  so Talk and Dictate were not exercised.
- No screen reader, and no human usability or accessibility study.
- LM Studio was not tested. Model answers are a handful of samples on one
  CPU-only machine, not a benchmark.
- The release workflow's publishing, signing and attestation steps have not
  run, because no release tag was pushed and no certificates exist.


## End-user readiness session (2026-10-12)

This change added a first-run screen that finds a local model server, plain-language errors, an app icon, crash reports, and installable packages for all three systems. It also fixed a macOS-only race in `james-loopback` that CI exposed.

**Automated checks** on Linux x64 with Rust 1.97.0 stable:

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed |
| `cargo test --workspace --all-targets --locked` | 138 passed, 1 ignored (the opt-in screen capture smoke test) |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed |

New tests: four in `james-models` (servers found in candidate order with the right route per provider, a 404 page and a non-model JSON reply ignored, default ports, embedding models skipped when choosing a default); four in the desktop (the "can't reach" message and passthrough of server errors, choosing a server with a chat model, a panic on a worker thread writing `crash.log`, the bundled icon decoding); one in `james-loopback` (arming a read after Stop reports the Stop).

**UI session.** A debug build ran under Xvfb (1400×1000, Mesa software OpenGL) with a scratch `HOME`. Python scripts played LM Studio on port 1234 (an empty `/v1/models`) and Ollama on port 11434 (`/api/tags` listing `nomic-embed-text:latest` and `gemma3:4b`, and a streamed `/api/chat`). Input came from `xdotool`.

1. With no session file, Conversation showed the welcome card. **Find my model server** with nothing listening reported that no server answered on ports 1234, 11434 and 8080.
2. With only the empty LM Studio stand-in running, it reported "LM Studio is running on port 1234, but it has no models yet" and how to fix it.
3. With both running, it chose Ollama (the server with a model), selected `gemma3:4b` over the embedding model, and saved provider, port and model to `session.json`. A typed question streamed back a reply.
4. With the Ollama stand-in stopped, **Send** reported "James can't reach Ollama on port 11434. Make sure Ollama is running…".
5. With a `crash.log` placed beside the session file, the next launch showed the internal-error notice; **Dismiss and delete the report** removed the file.

**Packages.** `scripts/package-linux.sh` on Ubuntu 24.04 produced a `.deb` and archive whose checksums verified. `apt install` of the `.deb` resolved its dependencies, `desktop-file-validate` accepted the menu entry, and the installed `/usr/bin/james` opened its window under Xvfb, then `apt remove` removed it. The archive's `install.sh` installed into a scratch home, the installed copy ran `--version`, and `install.sh --uninstall` removed the binary, menu entry and icon.

In CI on GitHub-hosted runners: the Linux job built in an `ubuntu:22.04` container, installed both packages there and ran `james --version`; the Windows job built the installer with the new icon settings and the renamed portable executable; the macOS job built both architectures, assembled and ad-hoc signed `James.app`, verified the signature and both slices, and ran `--version`.

**Found along the way.** A Linux binary built on Ubuntu 24.04 records a non-weak requirement for `GLIBC_2.39` (from optional `pidfd_spawnp`), so it would not start on Ubuntu 22.04, Mint 21 or Debian 12; Linux packages are now built on 22.04. On macOS, Stop arriving between a read loop's abort check and arming the next read made `setsockopt` fail with `EINVAL`, which surfaced as an I/O error instead of a clean stop; this flaked `abort_from_another_thread_unblocks_a_waiting_read_at_once` in macOS CI and is fixed.

**What this does not establish.** The model servers were scripts, not LM Studio or Ollama. No Windows or macOS installer or app was opened interactively, so SmartScreen, Gatekeeper, **Open Anyway**, permission prompts, Start menu and Applications launch remain unrecorded. The Intel slice of the macOS app was compiled, not run. No real Linux desktop (X11 or Wayland session, GPU driver) launched the packaged app.

## Streaming and Stop session (2026-10-12)

This change moved every network request onto `james-loopback`, a new dependency-free HTTP/1.1 client whose only address input is a port. It also made model replies stream and made **Stop** close the connection.

**Automated checks** on Linux x64 with Rust 1.97.0 stable, at the commit that added this section:

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed |
| `cargo test --workspace --all-targets --locked` | 128 passed, 1 ignored (the opt-in screen capture smoke test) |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed |
| Both `james-core` examples | Passed |
| Rust 1.85.0: `james-core` tests, Clippy on `james-core`, `james-loopback`, `james-models`, `james-lab` | Passed |

New tests: 21 in `james-loopback` (framing split across arbitrary writes, truncated bodies, header/body/line caps, idle and total timeouts, abort latency, redirects returned not followed, malformed and unsupported responses, IPv6 loopback fallback); 10 more in `james-models` (fragmented SSE across a UTF-8 boundary, NDJSON ordering, malformed frames, mid-stream disconnect, clean end without `[DONE]`, oversized replies, in-stream server errors, abort ending a stalled stream in under two seconds); one in `james-lab`; one in the desktop for a Stop pressed before the connection exists. Every server in these tests is a scripted socket on a random loopback port.

**UI session.** A debug build ran under Xvfb (1400×1000, Mesa software OpenGL, `libxkbcommon-x11-0` installed) with a scratch `HOME`. A Python script on `127.0.0.1:8080` played an OpenAI-compatible server that streams one word every 0.35 seconds over chunked `text/event-stream`. Input came from `xdotool`; the composer needed an explicit `windowfocus` because no window manager ran, and synthetic Ctrl+Enter did not register, so **Send** was clicked.

1. The request arrived with `stream: true`. The reply appeared in the Conversation word by word, with 11 words visible after 3.5 seconds and 21 after 6.5 seconds.
2. **Stop** was clicked mid-reply. The server's next-but-one write failed with a broken pipe 0.7 seconds later, after 50 words. The 50 words stayed in the Conversation and in `session.json`, followed by "[Stopped by you before the reply finished.]".
3. A second question, with the server at 0.04 seconds per word, completed: 68 words, saved whole with no label.
4. After a restart, both replies were restored and the status line read "Ready · mock-model on llama.cpp server at 127.0.0.1:8080". The Talk button rendered as text; the microphone emoji that showed as an empty box is gone.

**What this does not establish.** The server was a script, not LM Studio, Ollama or llama.cpp, so real streaming framing from those products is still unrecorded. Whether a real server stops generating when James closes the connection is that server's behavior and was not measured. No Windows or macOS UI run was made. This replaces the 90-second total read timeout recorded below with a 120-second idle limit between pieces of a reply, but no slow real model was rerun to confirm it.

## README screenshot session (2026-10-11)

The two README screenshots come from a real run of the desktop app, not mockups.

- **Build and display:** `cargo build -p james-desktop --locked` (debug) at commit `307f2b2` on a 4-vCPU Linux x64 cloud VM with no GPU. The app ran under Xvfb at 2240×1640 with `WINIT_X11_SCALE_FACTOR=2` and Mesa software OpenGL (`LIBGL_ALWAYS_SOFTWARE=1`, `LP_NUM_THREADS=1`). It first panicked at startup because `libxkbcommon-x11.so` was missing; installing the `libxkbcommon-x11-0` package fixed it.
- **Model server:** llama-cpp-python 0.3.36 (`python -m llama_cpp.server`, which wraps llama.cpp) on `127.0.0.1:8080`, serving `Qwen2.5-3B-Instruct` Q4_K_M from the official Qwen GGUF repository with `--n_threads 3`.
- **Flow:** in **Settings**, the *llama.cpp server* provider on port 8080 → **Discover models** returned `qwen2.5-3b-instruct` → selected and saved → **Conversation**. Three questions were typed into the composer with `xdotool` and sent with **Send**. The replies shown are the model's unedited output, which James saved to its session file.
- **Not exercised:** screen capture, Guide me, voice, computer input and the Lab connection.

What the session found:

1. **90-second read timeout.** With `Qwen2.5-1.5B-Instruct` generating about 2.4 tokens/s, a reply of about 260 tokens took 100 s and James reported `timed out reading response`. The error appeared in the status line and the question stayed in the history, which is correct behavior. Slow CPU-only machines can still hit this limit.
2. **CPU contention.** When llama.cpp used all four threads, the software-rendered UI's repaints during a pending request cut generation from 18.8 to about 2 tokens/s. Three threads restored it to 9.9 tokens/s (1.5B) and about 6 tokens/s (3B). This is a property of this no-GPU test host, not a measured desktop result.
3. **Small models answered poorly.** `Qwen2.5-0.5B-Instruct` answered "Yes" when asked whether a simulator finding is real-world evidence. `Qwen2.5-1.5B-Instruct` once claimed that *it* retains authority over its actions. Those transcripts were not used. Only the 3B transcript appears in the README. This reinforces the README's point that model claims carry no authority; the code enforces approval.
4. **Missing glyph.** The 🎙 on the **Talk** button renders as an empty box with the bundled egui fonts on Linux.

This is a single observed session, not a benchmark of model quality.

## v0.3 integration pass (2026-10-11)

### Delayed-action expiry regression fix

The UI's original five-second countdown now has a two-second dispatch grace period.
The same approval window reaches the worker and is checked before intent recording
and input handoff. Monotonic and wall clocks reject overdue resume, backward wall-clock
changes and a capture that becomes stale during the countdown. Expiry clears the
capture and requires a new review. All 11 local desktop tests passed, including five
new regression cases for timing, suspend/clock changes and expired worker dispatch;
the tests use simulated clocks and effects and send no real computer input.

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
