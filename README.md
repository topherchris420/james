<div align="center">

# James

### Your local AI companion. Your computer. Your call.

**Talk to a local model. Bring your screen into the conversation. Explore R.A.I.N. Lab research. Review every computer action before it happens.**

[![Core CI](https://github.com/topherchris420/james/actions/workflows/ci.yml/badge.svg)](https://github.com/topherchris420/james/actions/workflows/ci.yml)
[![Desktop CI](https://github.com/topherchris420/james/actions/workflows/desktop.yml/badge.svg)](https://github.com/topherchris420/james/actions/workflows/desktop.yml)
[![MIT license](https://img.shields.io/badge/license-MIT-0B5D63)](LICENSE)
[![Windows preview](https://img.shields.io/badge/release-Windows%20preview-555555)](docs/VALIDATION.md)

[**Preview builds**](https://github.com/topherchris420/james/actions/workflows/desktop.yml) · [**Quick start**](#start-here) · [**Platform support**](docs/PLATFORM_SUPPORT.md) · [**Release gates**](docs/RELEASE_READINESS.md) · [**Meet R.A.I.N. Lab**](https://github.com/topherchris420/lop-nur-twin)

</div>

---

> **Intelligence may propose. Evidence must be checked. Authority stays with the human.**

James is an open-source desktop companion built in Rust by [Vers3Dynamics](https://vers3dynamics.com/). It connects to a model **you run locally**, helps you understand what is on your screen, holds spoken conversations and guides you when you are stuck, and lets you inspect a proposed click, key press, or text insertion before choosing **Allow once**.

It is also the intended **desktop-facing counterpart to James in [R.A.I.N. Lab](https://github.com/topherchris420/lop-nur-twin)**, the simulated research environment. The connection is useful today but deliberately narrow: James can read the Lab's status and recent research context. It cannot start experiments, direct agents, or alter Lab state.

**The idea is simple:** the model can be helpful without being the authority.

## At a glance

| | v0.3 cross-platform foundation |
| --- | --- |
| **What it is** | An unsigned **Windows x64 preview**, with experimental CI archives for the Linux/macOS runner architectures. macOS/Linux have screen capture, speech and whisper.cpp listening (not yet device-validated); computer input is Windows-only |
| **Inference** | Select LM Studio, Ollama (native API), llama.cpp server, or an OpenAI-compatible local server on `127.0.0.1`; **you provide and load the model** |
| **Desktop context** | Explicit primary-screen capture and optional image attachment; **Guide me** for next steps on screen |
| **Voice** | **Talk** (hands-free conversation), Dictate and read-aloud. Windows speech recognition on Windows; local [whisper.cpp](https://github.com/ggml-org/whisper.cpp) on macOS/Linux. Voice can never approve actions |
| **Computer control** | **One reviewed action at a time**: click, allowlisted key, or printable text; no automatic action chain |
| **Research connection** | Read-only access to R.A.I.N. Lab's actual local status and discovery APIs |
| **Validation** | Previously verified Windows x64 preview build; new provider and cross-platform changes require passing CI and real-device acceptance tests |
| **Not yet proven** | Real-model response quality, end-user usability, accessibility, clean-machine installer lifecycle, reliable targeting of a specific app |

**This is a developer preview, not a production-ready autonomous desktop agent.** GitHub Actions tests and build artifacts are evidence of functioning code paths, not proof of flawless real-world use. For the exact test record, see [VALIDATION.md](docs/VALIDATION.md).

## See the interaction

A single, understandable loop connects the pieces:

```text
                      YOU
                       |
               question or request
                       v
              +------------------+
              |      JAMES       |
              |  Rust desktop UI |
              +------------------+
                 |      |      |
          local  |      |      | optional, read-only
          model  v      |      v
           +----------+ | +-------------------+
           | LM Studio| | |    R.A.I.N. Lab   |
           | /v1/chat | | | status / research |
           +----------+ | +-------------------+
                        |
            screen + proposed action
                        v
              +------------------+
              |  REVIEW EXACTLY  |
              | what will happen |
              +------------------+
                    |       |
                  Deny    Allow once
                    |       |
                 no input  v
                      one Windows input
                           |
                      inspect result
                           |
                    acknowledge outcome
```

**The model does not get a blanket permission slip.** In the current desktop build, James requires a fresh screen capture, an exact review, a one-use approval, and explicit reconciliation before another computer action. The destination is the foreground Windows application, **not** a reliably locked target window, so you must verify focus yourself.

## What you can do

### 1. Ask a local model

Choose your installed local provider in Settings, discover the model IDs reported by its server, and chat with a loaded model. LM Studio, llama.cpp and custom compatible runtimes use `/v1/models` and `/v1/chat/completions`; Ollama uses its native `/api/tags` and `/api/chat` routes. Replies are currently **non-streaming**. Model listings do not establish vision or tool capability. The compatibility tests use local HTTP fixtures, not a benchmark of real-model quality. See [PLATFORM_SUPPORT.md](docs/PLATFORM_SUPPORT.md).

### 2. Make your screen part of the conversation

Choose **Capture primary screen**, inspect the preview, and explicitly attach it to a question. A vision-capable model is needed to interpret images. Capture is opt-in; James does not silently watch the desktop. Capture works on Windows, macOS (`screencapture`; grant Screen Recording) and Linux (gnome-screenshot, spectacle, grim or ImageMagick).

**Stuck?** Choose **Guide me**: James takes one snapshot and the model explains what matters on screen and up to three next steps *you* can take. Type a question first to ask something specific. Guidance never becomes an action proposal.

### 3. Review one action, then decide

Use the **Actions** page to prepare a click, supported key, or printable text insertion, or ask a vision-capable model to suggest one action. You see the exact proposal and rationale before choosing **Allow once** or **Deny**. No arbitrary shell commands, general file tools, unrestricted typing macros, or multi-step autonomous loops are exposed.

### 4. Talk with James

Choose **Talk** for a hands-free conversation: James listens, answers aloud, and listens again until you say "stop", press Esc, or stay silent for 15 seconds. **Dictate** puts one utterance into a reviewable draft instead, and **Read last answer** or spoken replies read answers with Markdown and links cleaned up. Spoken words only ever become chat text; voice has no path to approving an action.

| | Windows | macOS | Linux |
| --- | --- | --- | --- |
| Listening (Talk, Dictate) | Windows speech recognition | whisper.cpp + SoX or ffmpeg | whisper.cpp + parec, arecord, sox or ffmpeg |
| Spoken answers | Installed Windows voice | `say` | espeak-ng or spd-say |

On macOS and Linux, listening runs [whisper.cpp](https://github.com/ggml-org/whisper.cpp) on your computer. James records only until you pause, never sends silence to whisper, and deletes each utterance's temporary audio immediately. **Settings → Voice → Set up listening** finds `whisper-cli` and a model automatically and gives copyable commands (macOS: `brew install whisper-cpp sox`, then a ~150 MB `ggml-base.en.bin` model download). Endpointing is covered by synthetic-audio tests; recognition quality and microphone compatibility still need hands-on evaluation.

### 5. Bring laboratory research into dialogue

Connect to the separately running R.A.I.N. Lab and optionally attach a bounded snapshot of recent work to a model conversation. James treats laboratory output as **research context, not instructions or proven external facts**. Simulator findings remain simulator findings.

## Start here

### Option A: Windows build

1. Open the [**James desktop build workflow**](https://github.com/topherchris420/james/actions/workflows/desktop.yml).
2. Open a **successful run on `main`** and find its **Artifacts** section.
3. Download either:
   - **`James-Windows-x64-installer`:** extract the artifact ZIP and run the versioned `*-setup.exe` inside it for a per-user install.
   - **`James-Windows-x64`:** extract and launch `james.exe` without an installer.
4. Start a local provider: **LM Studio** (usually port **1234**), **Ollama** (11434), or a compatible **llama.cpp** server (often 8080), with an appropriate model loaded.
5. In James, open **Settings**, select that provider, enter its local server port, choose **Discover models**, then select its actual model identifier.
6. Open **Chat** and send your first message with **Send** or **Ctrl+Enter**.

**Before installing:** these builds are **unsigned Windows x64 previews**. They contain neither LM Studio nor model weights, and GitHub workflow artifacts may require sign-in and expire after 30 days. Installation, upgrade, and uninstall have **not** been tested on a clean Windows machine. Read the [installer guide](docs/INSTALLER.md) before distributing a build.

### Option B: Run from source

Install current stable Rust and the native linker/build tools. On Windows, use the MSVC toolchain and Visual Studio C++ Build Tools.

```bash
git clone https://github.com/topherchris420/james.git
cd james
cargo run -p james-desktop --locked
```

James supports OpenAI-compatible `/v1` endpoints and Ollama's native `/api` routes against numeric loopback only. You can check an already-running LM Studio instance on your **own** computer using `pwsh -File scripts/test-local-provider.ps1 -Provider LMStudio`; add `-Model '<listed model ID>'` to perform one real text-inference request. If you want to inspect the authorization contract without any actual desktop input:

```bash
cargo run -p james-core --locked --example approval_demo
cargo run -p james-core --locked --example supervised_demo
```

Both examples use simulated actions. Read the [desktop guide](docs/DESKTOP.md) for controls and troubleshooting.

## A demonstration worth running

The most important next test is not a larger feature checklist. It is a complete, observable conversation with the system:

1. **Conversation:** ask James a normal question through an actual loaded LM Studio model.
2. **Vision:** capture the primary monitor, opt in to attaching it, and ask James what it sees.
3. **Proposal:** request **one** harmless input action in a disposable test application.
4. **Review:** inspect the exact proposed action and choose **Allow once**. Verify the foreground target before dispatch.
5. **Reconcile:** inspect what happened, then acknowledge the result inside James.
6. **Research:** start R.A.I.N. Lab separately, connect James to its local service, and ask about an existing research result using explicitly enabled Lab context.

**This is a proposed hands-on acceptance test, not a completed validation claim.** Use non-sensitive windows and disposable test data. Record the model, versions, Windows configuration, action outcome, failures, and a short demo only after the sequence has genuinely succeeded. See the [benchmark protocol](docs/BENCHMARKS.md) and [validation ledger](docs/VALIDATION.md).

## How approval works

The supervision logic lives in [`james-core`](crates/james-core); the desktop input boundary lives in [`review.rs`](crates/james-desktop/src/review.rs).

1. **Prepare:** James accepts at most one allowlisted proposal, with a concrete target and arguments.
2. **Review:** you inspect the captured screen, exact input, and rationale. An approval applies to **one** action.
3. **Expire:** old captures, delayed dispatch, or a backward wall-clock change invalidate the approval. The UI provides a five-second focus countdown plus a bounded dispatch grace period.
4. **Record first:** James writes and synchronizes an action-intent marker before passing input to the Windows helper.
5. **Reconcile:** an interrupted, uncertain, or even apparently successful input blocks another action until you inspect the target and acknowledge it.

The marker is a conservative recovery gate, **not** a complete transactional event journal or proof that the target received the intended input. James has no global emergency keyboard shortcut, guaranteed foreground-app targeting, OS sandbox, or automatic outcome verification. **Stop** cancels pending work; it cannot undo input already sent. Read [SUPERVISION.md](docs/SUPERVISION.md) and [SECURITY.md](SECURITY.md) for the boundaries and remaining risks.

## R.A.I.N. Lab, through James

R.A.I.N. Lab is a separate research environment inside [Lop Nur Twin](https://github.com/topherchris420/lop-nur-twin). James is intended to be its conversational desktop counterpart: a place to interrogate hypotheses, examine evidence, and decide what should happen next.

**What works now:** the `james-lab` crate calls the local Lab's `GET /api/rain/status` and `GET /api/rain/discovery` routes and can attach recent results to chat with your permission.

**What does not work yet:** shared live agent memory, laboratory experiment initiation, multi-agent coordination from the desktop, or automated scientific claim verification.

To try the bridge:

```bash
git clone https://github.com/topherchris420/lop-nur-twin.git
cd lop-nur-twin
npm ci
npm run dev
```

Use the port printed by the Lab (normally **5173**). In James, open **Lab**, enter that port, choose **Connect to Lab**, and explicitly enable context attachment if wanted. Experiments, model setup, and research authorization remain in R.A.I.N. Lab. The Lab may be reachable but idle; that is **not** evidence an experiment is running.

The actual API contracts, version requirements, size limits, and research provenance are documented in [LAB.md](docs/LAB.md).

## Built as boundaries, not a monolith

| Rust crate | Responsibility |
| --- | --- |
| [`james-core`](crates/james-core) | One-use approvals, policy checks, budgets, cancellation, supervised action contracts |
| [`james-models`](crates/james-models) | Loopback provider selection, bounded OpenAI-compatible/Ollama text and optional image requests |
| [`james-platform`](crates/james-platform) | Screen capture and speech (Windows, macOS, Linux), whisper.cpp listening (macOS, Linux), and constrained Windows desktop input |
| [`james-lab`](crates/james-lab) | Read-only status and recent research from R.A.I.N. Lab |
| [`james-desktop`](crates/james-desktop) | Native `eframe`/`egui` UI, settings, local session state, review workflow |

The application is written in **Rust**, with fixed **PowerShell/Windows component helpers** for native platform functionality. Calling James a Rust desktop application does not mean every OS interaction is implemented solely in Rust.

The workspace forbids unsafe Rust and has cross-platform core CI. macOS/Linux helpers call standard system tools resolved from fixed system directories, never through a shell; computer input remains Windows-only. For design rationale, see [ARCHITECTURE.md](docs/ARCHITECTURE.md). Some architecture and supervision documents retain earlier prototype milestones; use the [v0.3 validation record](docs/VALIDATION.md) for current implementation evidence.

### Reproduce the checks

```bash
cargo fmt --all -- --check
cargo test --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Desktop CI now targets Windows, Ubuntu and macOS, with experimental portable Unix archives when the respective host build succeeds. The separate core matrix also covers macOS. **CI compilation does not establish a usable installation, native screen/input/voice support, or an actual-device test.** See the [platform matrix](docs/PLATFORM_SUPPORT.md) and [acceptance protocol](docs/ACCEPTANCE_TESTS.md).

## Privacy is a behavior, not a badge

- **Loopback only:** model and Lab clients accept numeric local addresses; redirects, environment proxies, and automatic cloud fallback are disabled. A local service may still forward or log your data, so inspect its configuration.
- **Explicit capture:** James does not store screenshot image files or raw microphone audio. Where a macOS/Linux tool requires a file (screen capture, whisper.cpp audio), it uses a private owner-only temporary folder that is deleted immediately; audio never leaves your computer. If you choose to attach a screenshot, the selected model service receives it.
- **Plaintext history:** conversation text and settings are saved unencrypted at `%LOCALAPPDATA%/Vers3Dynamics/James/session.json` on Windows.
- **Action record:** `action-intent.json`, beside the session file, can contain the **exact text** approved for insertion and persists until acknowledgement.
- **No secure erase:** clearing conversation history is not forensic deletion and does not independently remove unresolved action markers.

Do not use the preview with sensitive desktop content or credentials until you understand the local server, retention, and foreground-input risks. Read the complete [security scope](SECURITY.md).

## What comes next

James's highest-value work is **reliability and proof**, before autonomy:

| Order | Work | Evidence required |
| --- | --- | --- |
| **01** | Real LM Studio text, vision, and harmless-action evaluation | Reproducible end-to-end session with failures recorded |
| **02** | Streaming responses and network-level cancellation | Malformed/disconnected stream fixtures and cancellation checks |
| **03** | SQLite history and a crash-tested action journal | Restart, interruption, rollback, and replay-suppression tests |
| **04** | Window-specific targeting and better accessibility | Real application focus, scaling, and assistive-technology tests |
| **05** | Clean-machine installer verification, signed builds, short visual demo | Install/upgrade/uninstall matrix and an authentic recorded walkthrough |
| **Later** | Scoped tools, MCP interoperability, coordinated research workflows | Deterministic gates, human authorization, provenance, and measurable benefit |

A wider agent runtime should arrive **after** those gates, not replace them. The longer-term [roadmap](docs/ROADMAP.md) is a plan, not a list of shipped features.

## Contribute

Bug reports, platform tests, model fixtures, accessibility findings, documentation corrections, and reproducible failure cases are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md), [PROVENANCE.md](PROVENANCE.md), and the [research notes](docs/RESEARCH.md). Keep reference software, recovered assets, and other people's proprietary material out of this repository.

---

<div align="center">

**The Lab explores. James helps us understand. Humans decide.**

Built openly by [Vers3Dynamics](https://vers3dynamics.com/) · [MIT license](LICENSE)

</div>
