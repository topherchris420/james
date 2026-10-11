<div align="center">

# James

### A local AI companion for research, conversation, and actions you approve.

[![Core CI](https://github.com/topherchris420/james/actions/workflows/ci.yml/badge.svg)](https://github.com/topherchris420/james/actions/workflows/ci.yml)
[![Desktop CI](https://github.com/topherchris420/james/actions/workflows/desktop.yml/badge.svg)](https://github.com/topherchris420/james/actions/workflows/desktop.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-0B5D63)](LICENSE)
[![Windows preview](https://img.shields.io/badge/stage-Windows%20preview-555555)](docs/DESKTOP.md)

[**Download Windows builds**](https://github.com/topherchris420/james/actions/workflows/desktop.yml) · [**Desktop guide**](docs/DESKTOP.md) · [**R.A.I.N. Lab**](https://github.com/topherchris420/lop-nur-twin) · [**Contribute**](CONTRIBUTING.md)

</div>

Bring a local model, a question, and the context you choose. James is an open-source Rust desktop companion for chatting with your model, asking about a screenshot, dictating a draft, or bringing R.A.I.N. Lab research into a conversation. Ready to act? James shows you the specific click, key, or text first. Nothing is sent to the desktop until you choose **Allow once**.

> Intelligence may propose. Evidence must be checked. Authority stays with the human.

Try James as an unsigned Windows preview, available in portable and installer builds. Real-model text and vision quality, speech accuracy, accessibility, and clean-machine install/upgrade/uninstall validation remain to be checked. See the [validation record](docs/VALIDATION.md) for what has actually been checked.

## What you can do

| Feature | Current behavior |
| --- | --- |
| **Local-model chat** | Pick a loaded model and start a conversation. Text and image requests use an OpenAI-compatible local endpoint. Replies are non-streaming; LM Studio and model weights are installed separately. |
| **Screen context** | Show James what you are looking at: capture the primary monitor, inspect the preview, then choose whether to attach it to your next question. Image requests require a vision-capable model. |
| **Reviewed computer input** | Turn a screenshot into a proposed click, key, or text insertion, then inspect and approve each action before it reaches the foreground app. |
| **Windows speech** | Say it as a draft, review the recognized words, and decide what to send. You can also hear the last answer or enable spoken replies with installed Windows speech components. |
| **R.A.I.N. Lab context** | Check in on the separate local Lab and bring recent research context into chat when you choose. The connection is read-only. |
| **Local history** | Save conversation text and settings locally between sessions. Storage is plaintext; see [data and privacy](#data-and-privacy). |

Screen capture, computer input, and speech require Windows. Other platforms report those capabilities as unsupported. James does not yet provide autonomous action loops, arbitrary shell or file tools, general MCP integrations, or multi-agent desktop coordination.

## Get started on Windows

### 1. Download and launch

Ready to try James? Open the [desktop build workflow](https://github.com/topherchris420/james/actions/workflows/desktop.yml), choose a successful run of `main`, and download a build from its **Artifacts** section. GitHub may require sign-in; artifacts expire after 30 days.

| Artifact | How to use it |
| --- | --- |
| `James-Windows-x64-installer` | Extract the archive and run `James-<version>-windows-x64-setup.exe`. Setup installs for the current user and adds a Start menu entry. A checksum is included. |
| `James-Windows-x64` | Extract the archive and open `james.exe` to run the portable build. |

Use a recent build containing the static Windows runtime fix; older preview artifacts require the Visual C++ runtime. Builds are unsigned and do not include a model server, model weights, or R.A.I.N. Lab. See the [installer guide](docs/INSTALLER.md) for packaging and checksum details.

### 2. Connect a model

1. Start LM Studio, load a model, and enable its local server.
2. In James, open **Settings** and enter the server port, usually **1234**.
3. Choose **Discover models**, select a loaded model, and save settings.
4. Open **Chat**, write a question, and choose **Send** or press **Ctrl+Enter**.

Choose a model with image support before sending screenshots or requesting action suggestions. Then ask away. James uses `/v1/models` and `/v1/chat/completions`; protocol tests use mock servers and do not establish real-model inference quality.

### 3. Add context when you need it

- **Screen:** choose **Capture primary screen**. James minimizes and waits three seconds so you can show the target application. Review the image and opt in to attaching it to your next question.
- **Voice:** choose **Dictate**, then review the recognized text in the draft before sending. Recognition depends on your installed Windows language components, microphone permissions, and device configuration.
- **Lab:** start the separate [R.A.I.N. Lab](https://github.com/topherchris420/lop-nur-twin) development server. In **Lab**, enter its port, normally **5173**, and choose **Connect to Lab**. Enable context attachment when needed. Follow the [Lab setup guide](docs/LAB.md) for upstream configuration.

Lab connections do not authorize, start, stop, or modify research experiments. Research approvals stay in the Lab, and simulator findings retain their original evidence limits.

## How action review works

1. Capture the target screen and prepare one input in **Actions**, select a point in the preview, or choose **Suggest one action** with a vision model.
2. Inspect the exact proposed click, key, or text and choose **Allow once** or **Deny**.
3. Allowing starts a **five-second countdown**. Focus the intended application during that time. **Stop** or **Escape inside James** cancels; no global emergency shortcut is registered.
4. Inspect the result in the target application, then acknowledge it in James to clear the recovery block before another action.

Approvals expire if dispatch is more than two seconds overdue, the wall clock moves backward, or the capture becomes older than 60 seconds. An expired review requires a new capture and approval. The desktop uses `james-core` with a one-action budget and records intent before handing input to Windows.

The persistent action marker blocks further input after dispatch, interruption, or restart until you inspect and acknowledge the result. James does not automatically retry an uncertain action. This is a recovery gate based on your inspection; automatic outcome verification and a full durable event journal are future work.

Input goes to the foreground application. A click or key can submit or delete data, so keep the target consistent with the reviewed capture. Guaranteed application targeting and OS sandboxing are not implemented. See the [desktop guide](docs/DESKTOP.md) and [supervision contracts](docs/SUPERVISION.md) for details.

## Build from source

Install **Rust stable** and the native linker/build tools for your platform. On Windows, use the MSVC toolchain with Visual Studio C++ Build Tools. A fresh checkout needs Cargo registry access to resolve the workspace's GUI and HTTP dependencies.

```sh
git clone https://github.com/topherchris420/james.git
cd james
cargo run -p james-desktop --locked
```

Run the project checks before contributing:

```sh
cargo fmt --all -- --check
cargo test --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

The dependency-free core retains a Rust 1.85 minimum; the desktop dependency set uses current stable Rust. To explore the core without launching the GUI:

```sh
cargo run -p james-core --locked --example approval_demo
cargo run -p james-core --locked --example supervised_demo
```

Both examples simulate actions and perform no desktop input. On Windows, `scripts/check.ps1` runs formatting, tests, Clippy, and the original approval demo; optional GNU toolchain configuration is documented in [validation](docs/VALIDATION.md).

### Workspace layout

| Package | Responsibility |
| --- | --- |
| [`james-desktop`](crates/james-desktop) | Native `eframe`/`egui` interface, conversation history, and integration of model, platform, and Lab clients. |
| [`james-core`](crates/james-core) | Authorization, one-use approvals, run budgets, cancellation, and supervised action lifecycle contracts. |
| [`james-models`](crates/james-models) | Bounded local model discovery and text/image completion requests. |
| [`james-platform`](crates/james-platform) | Windows screen capture, computer input, and speech helpers. |
| [`james-lab`](crates/james-lab) | Read-only Lab status, discovery, and bounded research context. |

## Data and privacy

Model and Lab clients connect only to numeric loopback addresses, with redirects and environment proxies disabled and no automatic cloud fallback. A local server can itself forward requests elsewhere or keep logs; check its inference and retention settings.

- **Conversation and settings:** stored unencrypted at `%LOCALAPPDATA%/Vers3Dynamics/James/session.json` on Windows.
- **Action marker:** the adjacent `action-intent.json` includes the exact input, including text to insert, until you acknowledge the action.
- **Screenshots and audio:** not saved by James. Explicit image and Lab attachments are sent to the selected model service, which may retain them.
- **History clearing:** overwrites local conversation history; it is not secure erase and does not independently clear an action marker.

See [storage details](docs/DESKTOP.md#storage-and-privacy) and the [security scope](SECURITY.md).

## Documentation and contributing

| Start here | More detail |
| --- | --- |
| Using James | [Desktop guide](docs/DESKTOP.md) · [Windows installer](docs/INSTALLER.md) · [Lab connection](docs/LAB.md) |
| Understanding the design | [Architecture](docs/ARCHITECTURE.md) · [Supervision](docs/SUPERVISION.md) · [Requirements](docs/REQUIREMENTS.md) |
| Assessing progress | [Roadmap](docs/ROADMAP.md) · [Validation](docs/VALIDATION.md) · [Benchmark protocol](docs/BENCHMARKS.md) |
| Contributing and provenance | [Contribution guide](CONTRIBUTING.md) · [Research](docs/RESEARCH.md) · [Source provenance](PROVENANCE.md) · [Security](SECURITY.md) |

Next work includes streaming and model-request cancellation, stronger outcome verification and recovery, application targeting, accessibility, real model and speech evaluations, installer lifecycle checks, and signed releases. Contributions should identify the concrete behavior they implement and the evidence used to validate it; start with [CONTRIBUTING.md](CONTRIBUTING.md).

James is part of [Vers3Dynamics](https://vers3dynamics.com/). It is an original Rust implementation informed by public product research; no reference product code, binaries, artwork, or recovered prompts are shipped. James's original files are licensed under [MIT](LICENSE).
