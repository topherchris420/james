# James desktop: first executable milestone

James is the evidence-oriented lead assistant from [R.A.I.N. Lab](https://github.com/topherchris420/lop-nur-twin). This repository now contains an original native desktop client and a localhost-only LM Studio adapter. **The client is not yet connected to R.A.I.N. research data or the existing Rust approval engine.**

## Windows: run a downloaded build

1. Install [LM Studio](https://lmstudio.ai/), download and load a model you are permitted to run locally, and enable **Developer > Local Server**. The default address is `http://127.0.0.1:1234`.
2. Open [James desktop CI](https://github.com/topherchris420/james/actions/workflows/desktop.yml), select a successful workflow run, and download the `James-Windows-x64` artifact. GitHub may require a signed-in account to download workflow artifacts.
3. Extract the artifact, double-click `james.exe`, enter the port from LM Studio, and choose **Discover models**.
4. Select a loaded model and send a text message. James uses the OpenAI-compatible `/v1/models` and `/v1/chat/completions` endpoints. A response is not streamed in this first milestone.

This is an **unsigned developer preview**, not a signed installer or an OS-certified release. Windows may display a security warning for unsigned executables. Verify that a build came from this repository's workflow before opening it. The archive does not bundle model weights or LM Studio, and no system installation is performed.

## Build from source

Rust **stable** and a target-specific native linker/build environment are required:

```sh
cargo test -p james-models -p james-desktop --locked
cargo run -p james-desktop --locked
```

On Windows, use the configured MSVC toolchain and Visual Studio C++ build tools. The core-only Rust contracts support Rust 1.85; the GUI uses a separately validated stable-toolchain dependency set. On Linux, native graphical libraries/display services must be available.

## Current behavior and boundaries

- **Real:** native Rust window, explicit numeric loopback port, model discovery, non-streaming Qwen/LM Studio chat, a research-oriented system prompt, conversation viewing, and an explicit clear-history operation.
- **Local storage:** model selection and chat messages are saved in plaintext under `%LOCALAPPDATA%/Vers3Dynamics/James/session.json` on Windows and `~/.james/session.json` where `HOME` exists. Clearing history overwrites the local session with an empty conversation. No encrypted-at-rest protection or secure erase is claimed.
- **Network:** the client constructs only `127.0.0.1` HTTP destinations. This does not prove an LM Studio backend won't itself use remote inference. Configure a genuine on-device model and verify its settings. The client makes no automatic cloud fallback.
- **Async responsiveness:** inference runs on a worker thread with bounded connect/read/write timeouts. Replies are delivered to the UI when complete. There is no token streaming, interruption, or in-flight cancellation yet.
- **Context:** the current system prompt identifies James as R.A.I.N.'s lead researcher but explicitly tells him he has **no access** to the Lab's files, papers or prior experiments. Conversation history is sent only to the selected localhost model service.
- **Authority:** no model tool calls, screen capture, voice, filesystem actions, research import, or automatic execution are enabled. The `james-core` authorization contracts are **not yet part of the desktop execution path**.

## Next milestones

1. Build a versioned read-only R.A.I.N. research context adapter. Require user selection and an evidence receipt for every imported artifact. Keep experiment observations separate from model inferences.
2. Add streaming response handling, explicit stop/cancel, history migrations, error surfaces, and cross-platform accessibility checks.
3. Wire staged tool requests through the genuine `james-core` supervisor, with a durable intent/outcome store, OS-level resource confinement, and human review. No write or desktop-control tool can ship without this gate.
4. Package, sign, and independently validate Windows builds. Build downloadable installers only after a reproducible release process exists.

No reference binaries, proprietary code or artwork were copied into James. The separately provided Windows reference installer is not a prerequisite to building or running this application.
