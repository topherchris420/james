# Changelog

James follows [semantic versioning](https://semver.org/). Before 1.0, a minor
version (0.4, 0.5…) may change behaviour or saved-data formats; each entry
says when it does and how older data is handled.

Releases are published only from an explicit `v<version>` tag, by the
workflow described in [docs/INSTALLER.md](docs/INSTALLER.md). Versions before
0.4.0 were never published as releases. A version with a suffix, such as
`0.4.0-preview.1`, is published as a prerelease.

## Unreleased

### Changed

- **Guide me's step numbers are off by default.** Turn them on in Settings →
  Guide me. Your model decides where each number goes. In a test with
  Gemma 4 E4B in LM Studio, on synthetic screens with known button
  positions, the written steps were right but the numbers landed 150 to 270
  pixels from the buttons they meant. When numbers are shown, James now says
  the model placed them and they may be off. With them off, Guide me sends
  exactly the request it did before step numbers existed.

## 0.4.0-preview.2 — 2026-10-12

A second developer preview: Guide me marks its steps, and screen and voice
features work with models whose chat template accepts one system message.

### Added

- **Guide me marks its steps.** Each suggested step is numbered on your
  screenshot, under the answer and on the Screen page, where the model says
  its button, field or link is. The marks are drawn on James's copy of the
  picture, never over your screen, and they only point: you still take each
  step yourself. The model gives them on one extra line of its answer, which
  James removes before the answer is shown, saved or read aloud; marks off
  the picture, beyond step three or repeated are ignored. A model that leaves
  the line out still gives the same written steps.

### Fixed

- **Guide me, Plan and voice conversations failed with some models**, such as
  Qwen 3.5 in LM Studio, whose chat template accepts only one system message
  ("System message must be at the beginning"). James now sends all its
  instructions as one system message.

## 0.4.0-preview.1 — 2026-10-12

The first published build of 0.4.0, released as a prerelease because it is a
developer preview.

The first version prepared for people who are not developers. See
[docs/END_USER_READINESS.md](docs/END_USER_READINESS.md) for what has and has
not been verified; the release classification is **developer preview**.

### Added

- **Guided first-run setup** in five steps: what James is, how to get LM
  Studio or Ollama on your operating system, Find my model server, a real
  test message, and what screen capture, permissions and reviewed input mean.
  Progress is saved after every step and resumes after a restart.
- **What a model can do**, asked of its own server: whether it reads images,
  whether it can chat, its context window and whether it is loaded (Ollama,
  LM Studio, llama.cpp). Screen features are switched off when the server
  says a model cannot read images.
- **Cloud models are flagged.** Ollama models that run on another computer
  are labelled, warned about, and never chosen automatically.
- **Conversation:** Markdown rendering with copyable code blocks, Copy, Ask
  again, Try again, a prominent Stop, a "thinking" indicator for reasoning
  models, example questions, and the model and app in the title bar.
- **Appearance and accessibility:** light, dark and system themes, high
  contrast, text size from 80% to 200%, reduced motion, keyboard shortcuts
  (F1 lists them) and screen-reader support through AccessKit.
- **Privacy controls:** keep conversations off disk, clear a conversation, or
  delete all James data (an unchecked action record is kept until checked).
  On macOS and Linux, James's folder and files are readable only by you.
- **Action recovery** shows exactly what an interrupted action was, where,
  when, and what is known about whether it happened, on every platform.
- **Windows target check:** a screenshot records the window in front; input
  is refused unless that same window is in front (and, for a click, under
  the point), and never sent to James itself.
- **R.A.I.N. Lab:** Find the Lab, plain-language results that say whether
  each is supported in the simulator and whether it was replayed, model
  hypotheses labelled as such, the Lab build, gentle retries while offline,
  and a preview of exactly what would be shared with the model.
- `cargo run -p james-models --example probe`: check any local model server
  from a terminal on any operating system.
- Linux packages include AppStream metadata for software centres; the
  Windows uninstaller asks whether to delete your conversations (default:
  keep them).
- Release builds can be Authenticode-signed (Windows) and signed and
  notarized with a Developer ID (macOS) when a maintainer configures real
  credentials; releases carry build-provenance attestations and a
  `release-metadata.json` that states whether signing happened. Windows
  signing works with Azure Artifact Signing, Microsoft's service that keeps
  the key in its own hardware (new certificates can no longer be issued as a
  `.pfx` file), or with an existing certificate file; setup is in
  [docs/INSTALLER.md](docs/INSTALLER.md#getting-a-windows-signing-identity).
- Every package includes `THIRD-PARTY-NOTICES.txt`: the license of each
  open-source crate built into that package and the texts the crate
  publishes, generated from `Cargo.lock` by `tools/notices`.
- **What's new after an upgrade.** From this version on, the first time an
  upgraded James starts it shows that version's notes from this changelog,
  built into the app. Got it, Esc or the close button dismisses them for
  good. James still never connects to the internet to look for updates.

### Changed

- Clear errors with a next step for: a server not running, a missing or
  unloaded model, a full context window, a model that cannot read images or
  chat, a model still loading, and a model that needs a sign-in. The
  server's own words are always shown too.
- **Screenshots wait for James to leave the picture** (Windows). After the
  three-second delay, the capture helper checks that none of James's windows
  still covers the screen, waits for the display to finish redrawing, and only
  then takes the picture. If James is still visible after 1.5 seconds, the
  picture is taken anyway and James says it may appear in it.
- Screenshots are scaled to at most 1600 pixels before a model sees them;
  suggested clicks are mapped back to full-screen coordinates for review.
- Conversation history is trimmed to fit a known context window, and James
  says when older messages were left out.
- A reply may take at most 30 minutes in total, in addition to the existing
  two-minute silence limit.
- The action record format is now `james-action-intent/v2` (id, time,
  screen, target window and outcome). A v1 record from 0.3 is still read and
  still blocks new actions until checked.
- The session file gains `keep_history`, `preferences`, `setup` and
  `last_version`. Sessions from 0.3 load unchanged; one with a model already
  chosen skips setup.

### Fixed

- The Lab's `report` field, a file path inside the Lab, is no longer shown
  as if it were the report's text.
- Characters the bundled font cannot draw no longer appear in the interface.
- The desktop needs Rust 1.88, not 1.85 as the README said; its crates now
  declare it and CI checks it.

### Security

- Files James writes are created owner-only and written atomically, with
  their folder synced; older, looser files are tightened when loaded.
- Model proposals are tested against duplicate fields, look-alike action
  names, key chords, action lists, self-approval fields and out-of-range
  numbers. None can produce an approval.

## Before 0.4.0 (unreleased development)

- **0.3.0:** screen capture, Windows voice and reviewed Windows input, live
  read-only Lab context and the Windows installer; then Ollama, llama.cpp and
  OpenAI-compatible providers, voice conversations, Guide me and whisper.cpp
  listening on macOS and Linux, the `james-loopback` transport (a port is the
  whole address), streamed replies with a Stop that closes the connection,
  Find my model server, crash reports, and packages for macOS and Linux.
- **0.2.0:** the native Rust desktop with LM Studio integration.
- **0.1.0:** the `james-core` supervision contracts: one-use approvals,
  policies, budgets and journaled dispatch.
