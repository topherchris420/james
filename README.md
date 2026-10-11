<div align="center">

# JAMES

### A local research companion, with human review before action.

[![CI](https://github.com/topherchris420/james/actions/workflows/ci.yml/badge.svg)](https://github.com/topherchris420/james/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT-0B5D63)](LICENSE)
[![Stage](https://img.shields.io/badge/stage-Windows%20preview-555555)](docs/DESKTOP.md)

[**R.A.I.N. Lab**](https://github.com/topherchris420/lop-nur-twin) · [**Windows builds**](https://github.com/topherchris420/james/actions/workflows/desktop.yml) · [**Desktop guide**](docs/DESKTOP.md)

</div>

James v0.3 is a **Windows desktop preview** with a native Rust interface, local-model chat, optional screen context, reviewed computer input, Windows speech, and a live read-only R.A.I.N. Lab connection. The project also includes a per-user Windows installer build and release workflow. It is unsigned; real-model quality, voice accuracy, and complete installer lifecycle validation remain work to do.

> Intelligence may propose. Evidence must be checked. Authority stays with the human.

## What works in this implementation

| Capability | Implemented behavior | Boundary |
| --- | --- | --- |
| Desktop interface | Native `eframe`/`egui` window with Chat, Screen, Actions, Lab and Settings views; local history | Windows preview; accessibility and broader platform validation remain incomplete |
| LM Studio / local AI | Loaded-model discovery, non-streaming text and image requests through OpenAI-compatible endpoints | LM Studio and model weights are separate; screen questions require a vision-capable model |
| Screen understanding | On-demand primary-monitor capture, visible preview and explicit attachment to the next question | One snapshot, not continuous monitoring; interpretation depends on the selected model |
| Computer control | One reviewed click, supported key, or printable-text insertion; optional model-proposed action | **Allow once** is required for every action; no autonomous action loop or shell tool |
| Voice | Dictate one utterance into an editable draft; read an answer aloud; optional spoken replies | Uses installed Windows speech components; recognizer/language availability and microphone access vary |
| R.A.I.N. Lab | Polls the actual local discovery API; displays current state and optionally attaches recent research context | Read-only; no research authorization, experiment control, or automatic shared memory |
| Windows installation | Inno Setup per-user installer, portable build, checksums and explicit-tag release workflow | No signing yet; installation, upgrade and uninstall still require hands-on lifecycle validation |

The model and Lab clients use numeric loopback addresses with no automatic cloud fallback. A local endpoint can itself forward requests elsewhere: verify the server's inference and logging settings. No reference product binaries, code, artwork or recovered prompts are shipped.

## Launch James

After these changes are merged and a build succeeds, the [desktop workflow](https://github.com/topherchris420/james/actions/workflows/desktop.yml) produces `James-Windows-x64` (portable) and `James-Windows-x64-installer` (setup plus checksum). This describes the workflow, not a claim that a v0.3 release is already published. GitHub may require sign-in to download artifacts.

Run `James-<version>-windows-x64-setup.exe` and choose **Install**, or extract the portable artifact and open `james.exe`. Setup installs for the current user and creates a Start menu entry. It does not bundle LM Studio, model weights or R.A.I.N. Lab. See [installer setup and validation](docs/INSTALLER.md).

Start LM Studio, load a model and enable its local server (usually port **1234**). In James, open **Settings**, choose **Discover models**, select the loaded model, and send a question. To understand screenshots or propose actions from them, select a model that supports image input.

For research context, start the separate [R.A.I.N. Lab](https://github.com/topherchris420/lop-nur-twin) development server and connect from James's **Lab** view, normally on port **5173**. Attachments are opt-in. See the [verified API and setup](docs/LAB.md).

## Review before input

Capture the primary screen, then select a point or prepare a key/text action in **Actions**. Alternatively, ask the vision model to **Suggest one action**. Inspect the exact proposed input and select **Allow once**. A five-second countdown gives you time to focus the target application. **Stop** or Escape inside James cancels; no global emergency shortcut is registered.

The desktop routes approved input through `james-core` with a one-action budget and records intent before dispatch. A persistent action marker blocks further actions until you inspect the target and acknowledge it. If James crashes or input fails, it does not automatically retry or claim the outcome is known. This is a conservative recovery gate, not automated verification of the target application's result or a full durable event journal.

Clicks and keys can submit, delete or otherwise change data in the foreground application. Review depends on the target still matching the captured screen. There is no process sandbox or guaranteed application targeting.

## Build from source

Install Rust stable and the native linker/build tools for your platform. Windows uses the MSVC toolchain and Visual Studio C++ Build Tools by default.

```sh
git clone https://github.com/topherchris420/james.git
cd james
cargo fmt --all -- --check
cargo test --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo run -p james-desktop --locked
```

Windows screen, input and speech capabilities report unsupported on other platforms. The core-only Rust contracts retain their Rust 1.85 minimum; the graphical dependency set uses current stable Rust. The original core demonstrations remain available as `approval_demo` and `supervised_demo`; those examples simulate actions.

## Data and validation

Conversation text and settings are stored unencrypted locally. The latest action marker also contains the exact input, including text to type. Screenshots and audio are not saved by James; an attached image is sent to the selected model server. The server may keep its own logs. Clearing local history is not secure erase. See [desktop storage details](docs/DESKTOP.md).

The Lab client was exercised against the actual upstream local server and returned a reachable runtime with idle discovery state. No research experiment was started. Local AI protocol tests use mock HTTP responses; they do **not** establish successful real LM Studio inference, vision quality or reliable action proposals. Hands-on microphone/recognition accuracy, computer-control behavior across target applications, and clean-machine installer lifecycle checks remain required. See [validation](docs/VALIDATION.md).

## Project boundaries and next work

R.A.I.N. Lab remains the authority for research, evidence and experiment approvals. James can read selected context without acquiring that authority. A simulator result stays simulator evidence, and model output remains a proposal.

Next work includes streaming and model-request cancellation, stronger outcome verification and recovery, application targeting, accessibility, model/vision evaluations, speech validation, installer lifecycle checks and signed releases. This preview does not implement general MCP tools, arbitrary file operations, autonomous desktop workflows or multi-agent desktop coordination.

- [Desktop guide](docs/DESKTOP.md)
- [Lab connection](docs/LAB.md)
- [Windows installer](docs/INSTALLER.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Supervision contracts](docs/SUPERVISION.md)
- [Roadmap](docs/ROADMAP.md)
- [Validation](docs/VALIDATION.md)
- [Security scope](SECURITY.md)
- [Provenance](PROVENANCE.md)
- [Contributing](CONTRIBUTING.md)

James is part of [Vers3Dynamics](https://vers3dynamics.com/). This is original Rust implementation informed by public product research, not a formally separated clean-room claim. **License: [MIT](LICENSE).**
