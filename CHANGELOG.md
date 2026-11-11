# Changelog

James follows [semantic versioning](https://semver.org/). Before 1.0, a minor
version (0.4, 0.5…) may change behaviour or saved-data formats; each entry
says when it does and how older data is handled.

Releases are published only from an explicit `v<version>` tag, by the
workflow described in [docs/INSTALLER.md](docs/INSTALLER.md). Versions before
0.4.0 were never published as releases. A version with a suffix, such as
`0.4.0-preview.1`, is published as a prerelease.

## Unreleased

### Added

- **Inspectable document answers.** Text/PDF attachments and selected workspace
  files receive request-local source IDs and line numbers. Expand model citations
  to read the supplied lines; missing, malformed and out-of-range references are
  disclosed. Location checks do not verify that a claim is true or supported.
- **Request evidence.** Inspect the latest chat's prepared messages, model,
  source snapshots, image-selection description, omitted history and outcome.
  Explicitly copy a JSON record or forget it. Records stay in memory and are
  never saved in conversation history. See [Request evidence](docs/REQUEST_EVIDENCE.md).
- **Local conversation search.** Filter retained history by literal words in
  titles and messages without contacting a model.

### Fixed

- Refuse fixed document/question context that exceeds the model's advertised
  window under the existing byte-budget heuristic, before consuming the draft
  or attachments. Previously only conversation history was trimmed. This is
  an estimate, not exact token accounting, and excludes image token costs.

### Earlier additions

- **Revise a copy.** Preview one small source file, send its frozen contents
  with revision instructions, review the complete output and create a new file.
  Existing source and destination files are never overwritten.
- **Reviewed Windows settings pages.** Choose a common settings page manually
  or ask the model to select one, then review its fixed destination before opening.
  James does not change the setting.
- **Configurable global shortcuts.** Edit summon, Talk and Dictate chords in
  Settings. Enabled chords must be distinct; a failed replacement keeps the
  previously registered chord.
- **Reviewed workspace context.** List a selected directory, preview up to three
  explicitly chosen text files, and attach their frozen contents to the next
  chat question. Each file is limited to 4 KiB; hidden files and links are not
  included. File contents are not saved in conversation history.
- **Windows media controls.** The floating companion can explicitly refresh
  now-playing information and request play/pause or next. Native media sessions
  have not yet been verified against real players.
- **Reviewed creation of a new text file.** Specify a directory, filename and
  instruction, then review the model's proposed contents before creation.
  Existing files are never overwritten; edits to existing files are not yet
  supported.
- **Reviewed dictation into another Windows app.** Opt in through the Dictate
  shortcut, focus the destination field, and review the transcript and target.
  After confirmation, return to the field during the three-second countdown.
  James checks the native window and control before sending Unicode input; the
  clipboard is unchanged. Custom controls can change internal focus, and native
  delivery still needs real-app verification.
- **Companion docking on Windows.** The floating companion can snap to the
  left or right edge of the primary display when its geometry is verified.
- **Desktop guidance on Windows.** Show screenshot marks and numbered Guide me
  steps over the primary display in a click-through window. It checks geometry,
  expires with the screenshot, and is hidden during capture. Model-placed marks
  may be wrong; native behavior still needs real-device verification.
- **Optional voice shortcuts.** Ctrl+Shift+V starts Talk and Ctrl+Shift+D starts
  Dictate (Command instead of Ctrl on macOS). Dictation ends on a
  pause; these are not hold-to-record shortcuts. Wayland remains unsupported.
- **Local reminders.** Due reminders open the floating companion while James
  runs. Optionally save them on this computer to resume after restart.
  They are not delivered as OS notifications.
- **Floating companion.** Settings can open a movable always-on-top companion
  with a simple timer. It is hidden during screen capture; the timer runs only
  while James is open and reports completion in the companion window.
- **File attachments.** Drop a file on James or use **Attach file** to enter a
  path. Chat supports UTF-8 text, extractable PDF text, PNG, JPEG and WebP.
  Files are limited to 16 MiB and extracted text to 24 KiB. Image-only PDFs
  need OCR outside James. Attachments are not saved in conversation history.
- **Conversation history.** New conversation archives the current chat; History
  resumes or deletes earlier chats. Up to 20 archives share the existing
  80-message/512 KiB retention budget. Turning off history removes current and
  archived conversation text from saved settings.
- **Compact companion and optional global summon.** Companion opens a smaller,
  always-on-top view. Enable Ctrl+Shift+Space (Command+Shift+Space on macOS) in
  Settings to summon it. Registration errors are shown; Wayland is unsupported.
- **Point and draw on screen context.** Add colored marks, undo or clear them,
  and inspect the selected region before sending. A failed crop stops the
  request. Talk preserves the typed draft and its screenshot attachment.
- **Korso reconstruction report.** [Static findings and replication coverage](docs/KORSO_REPLICATION.md)
  trace the inspected client behavior and identify remaining parity work.

- **Second-pass review of a proposed action.** Before you review a click, key
  press or typing that **Suggest one action** proposed, a second model pass
  looks at the same screenshot and writes a short note — flagging anything
  irreversible or off-target — shown with the action on the review card. You
  still approve every action yourself; the note is a prompt to look, not a
  gate. On by default; it costs one extra model request, so it adds a short
  wait, and can be turned off in **Settings → Reviewed actions**. James's local
  take on Korso's planner+reviewer coordination, kept inside James's
  one-approval-one-action model.
- **Choose which monitor to capture.** When more than one display is
  connected, the Screen context page shows a **Monitor** picker next to the
  capture button, so you can grab a screen other than the primary one. Vision
  questions and **Guide me** work on any monitor. Reviewed clicks stay
  primary-only — the input helper positions the cursor in primary-screen
  pixels — so **Suggest one action** and click-to-point are offered only for a
  primary-screen capture and explain why on another monitor. With a single
  display the picker stays hidden and nothing changes.
- **Ask about just part of the screen.** On the captured screenshot, drag a
  box over the area you mean; your next attached chat question then sends only
  that area to the model, not the whole screen. A size readout and a **Use
  whole screen** button show and clear the selection, which is outlined on the
  picture. Guide me and Suggest one action are unchanged — they still send the
  whole screenshot, because they rely on whole-screen coordinates. The crop is
  made in memory for the request only and never saved.
- **Choose the voice that reads answers aloud.** **Settings → Voice** now lists
  the text-to-speech voices installed on your computer (the Windows voices, or
  the macOS and `espeak-ng`/`speech-dispatcher` voices on the other systems) and
  reads new answers and **Talk** replies in the one you pick. The list loads off
  the interface thread so it never blocks; leaving it on **System default**, or
  choosing a voice the computer later removes, keeps the built-in voice. The
  choice is saved with your other settings. James still speaks only on your
  computer, with no cloud voice.
- **Copy conversation** and **New conversation** above the chat, reachable
  even after scrolling through a long answer. Copy produces a Markdown
  transcript without screenshots or the unsent draft. New conversation
  (Ctrl+N, or ⌘N on Mac) asks before clearing messages, draft, screenshot,
  attachments and any unapproved action proposal. Settings and unresolved
  approved-action records stay.

### Security

- **The review shows every character James will type**, in a scrollable
  box, instead of the first 200. The record of an unchecked action shows it
  the same way. Only the countdown, which points back to that review,
  shortens it.
- **Typed text can't hide or reorder what the review shows.** James and the
  Windows input helper both refuse the line and paragraph separators
  (U+2028, U+2029) and invisible formatting characters: zero-width
  characters, text-direction marks, embeddings, overrides and isolates, tag
  characters and fillers. Emoji, accents and every script still type. Window
  titles have the same characters replaced before they're shown.
- **A second, deliberate step for risky model proposals.** When a model
  proposes a click, typing, or Enter, Space, Delete or Backspace, Allow once
  stays off until **I checked what this does and where it goes** is ticked.
  Actions you set up yourself are unchanged.
- **A model's reason is one plain line in its own frame**, with line breaks
  and invisible characters removed, so it can't pass itself off as James's
  text on the review card.
- **The Lab snapshot goes only with chat questions.** Guide me and Suggest
  one action send the screenshot and the question alone, so text from
  whatever answers on the Lab port can't shape a proposed action.
- **Links in replies ask before opening.** A dialog names the site the link
  really goes to (a user name before `@` doesn't count), shows the full
  address, and warns when it carries a lot of text after the site name,
  which a model-written link could use to send conversation or screen text
  away. The link opens only from **Open in browser**; Esc or Cancel closes
  the dialog without stopping anything else.

### Fixed

- **Save failures stay visible** above every page, with **Retry saving**.
  A later status cannot hide them, and clearing a conversation or turning
  history off no longer reports success when writing the file fails.
- **Ask again** keeps an unsent draft and its screenshot attachment. The
  screenshot selected for that draft is never sent with the retry.
- **Long messages no longer disappear when saved.** A message over 32 KiB
  is shortened at a character boundary with a visible note, including when
  an interruption label puts a partial answer over the limit.
- **History stays reloadable.** The newest 80 messages are limited to
  512 KiB of text so escaped JSON stays below the 4 MiB read limit.
  Oversized settings leave the previous file intact. The file writer also
  enforces history-off itself, so callers cannot accidentally save messages
  when retention is disabled. Existing session files remain compatible.

## 0.4.0-preview.3 — 2026-10-12

Guide me's step numbers become a setting, off by default, after a real
model placed them well away from the buttons it meant.

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
