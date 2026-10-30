# Security scope

James 0.4.0 is an unsigned **developer preview**, not an OS sandbox. It talks to local model servers and an optional local Lab, captures the screen when asked, listens and speaks, and on Windows can send one reviewed click, key or piece of typing at a time. This page states what protects each of those and where the protection ends. The design rule throughout: intelligence may propose, evidence must be checked, and authority stays with the person.

## Network

Model and Lab requests go through `james-loopback`, whose only address input is a port. It connects to `127.0.0.1` or `::1` and cannot express another destination; there is no DNS, proxy, TLS or redirect following, and no other HTTP client exists in James. Header, line and body sizes are bounded, connections have connect, idle and total timeouts, and Stop shuts the socket down from the UI thread. James never contacts the internet: there is no telemetry, crash upload or automatic update check.

A loopback endpoint does not authenticate the process behind it, and it does not prove inference stays on this computer. Ollama, for one, can relay a model to its cloud service. James reads what the server reports: models it marks as remote are labelled "runs elsewhere", warned about before use, and never chosen automatically, but a server can relay without saying so. Use model apps you trust. No API keys are stored, and authenticated model servers are not supported.

**Find my model server** sends one model-list request to each of ports 1234, 11434 and 8080, only when the person presses it, and accepts only a well-formed model list. **Check what this model can do** reads the server's own capability endpoints, each bounded to five seconds and 1 MiB. Neither ever installs, starts, loads or downloads anything.

## Screen, voice and model input

Screen capture happens only on an explicit button press, after James hides for three seconds. The picture stays in memory, and it goes to the model only when the person attaches it or presses Guide me or Suggest one action. Guide me and Suggest one action send only the screenshot and the question: no conversation history and no Lab snapshot. On macOS and Linux, tools that need a file (screen capture, whisper.cpp) use a fresh owner-only temporary folder created with create-not-reuse semantics and removed immediately.

Voice can only become chat text; it has no path to approving or sending input. Model replies, screen text and Lab snapshots are untrusted data. Prompt instructions are not a security boundary: model output can at most propose one allowlisted action for review, and its stated reason is shown on one line, without line breaks or invisible formatting, as the model's words, not as a guarantee. A Lab snapshot goes only with chat questions, so Lab text never reaches the request that proposes an action.

Links in replies open only after James names the site, shows the full address and the person chooses **Open in browser**; only `http` and `https` addresses can open. A model can write any address, and an address can carry text from the conversation or the screen to that site, so James warns when one carries a lot of text after the site name. Apart from what a model app does with what it receives, an opened link is the only way something from James can leave this computer.

## Computer input (Windows only)

The host accepts one click, one allowlisted key or up to 1,000 characters of visible text on one line. Typed text may not contain control characters, the line and paragraph separators U+2028 and U+2029, or invisible formatting characters (zero-width characters, text-direction marks, embeddings, overrides and isolates, tag characters and fillers), which could hide or reorder what the review shows; James checks this before review and the Windows input helper checks it again. No command execution, shell, process launch, action sequence or agent loop is exposed to a model. A model's proposal is parsed strictly: several actions, text around the JSON, duplicate fields, unknown fields, look-alike letters, key chords, non-integer or out-of-range coordinates, and claims of approval are all refused before review (see the adversarial tests in `crates/james-desktop/src/review.rs`).

The review shows the exact action, including every character of any typing, the window it is meant for and the screenshot, and requires a capture less than 60 seconds old. When a model proposed a click, typing, or Enter, Space, Delete or Backspace (inputs that can press, send, submit or delete), **I checked what this does and where it goes** must be ticked before Allow once is enabled. **Allow once** creates the only `ReviewedAction`, a non-cloneable value consumed by one dispatch. A five-second countdown lets the person switch windows; **Cancel**, or Esc while James has focus, stops it. There is no global stop key. The approval expires if the countdown ends more than two seconds late or the wall clock moves backwards.

Before input, James writes `action-intent.json` with create-new semantics and flushes it (on Unix, also its folder, with mode 0600). An existing or unreadable record blocks new input. The record stays until the person confirms they checked the target, including after a restart, crash or uncertain failure, and no interrupted action is ever retried. This is a single-outstanding-action gate with human reconciliation, not a transactional journal, and James cannot verify that an input had its intended effect. Real power loss during a real Windows input has not been tested.

**Target window.** On Windows, capture records which window was in front (handle, process id, process name, title). Immediately before sending, the input helper refuses, and says nothing was sent, if James itself is in front, if a different window or process is in front, if the screen size changed, or if a click would land on a different top-level window. Limits:

- If capture could not tell which window was in front, only the James-in-front check applies; the review warns about this.
- A window that comes to the front in the milliseconds between the check and the input is not prevented.
- The same window may show different content than the screenshot; unchanged identity does not prove unchanged content.
- This is not process isolation: applications running with higher privileges, focus-stealing applications or input from other sources can still interfere. Clicks address the primary monitor only.

Input can submit, send or delete depending on the target, and sent input cannot be undone. Never approve based on a model's reason alone.

Windows helpers run fixed embedded PowerShell scripts through the system PowerShell path; user and model strings travel as JSON on standard input and are never interpolated into commands. Native input uses platform API interop inside those fixed scripts; the Rust workspace forbids `unsafe` code. The helpers are not sandboxed. CI parses every helper and compiles its embedded C# (`scripts/check-powershell.ps1`). On macOS and Linux, helpers are system tools resolved only from fixed system directories (never the search path) and run without a shell; a whisper.cpp program or model path entered in Settings must be an absolute path to an existing file and is trusted as the person's choice. James sends no clicks or keystrokes on macOS or Linux.

## Stored data

Everything James keeps is in one folder: `%LOCALAPPDATA%\Vers3Dynamics\James` on Windows, `~/.james` on macOS and Linux. It holds `session.json` (settings, setup progress and, unless turned off, the conversation), `action-intent.json` (the last approved action, including any text to type, until checked) and `crash.log`.

- On macOS and Linux the folder is 0700 and every file 0600, written atomically; folders and files left by older versions are tightened on launch. On Windows, James relies on the default per-user permissions of `%LOCALAPPDATA%` and does not change or check them.
- Nothing is encrypted at rest. Full-disk encryption (BitLocker, FileVault, LUKS) protects the folder if the computer is lost; another program running as the same user can read it.
- **Keep my conversations on this computer** can be turned off, which removes the saved copy. **Clear conversation…** and **Delete all James data…** remove files after confirmation, but deletion is not secure erasure: backups and disk recovery may find old copies. Delete all keeps an unresolved action record until the person checks it.
- Screenshots and raw audio are never saved by James. What is sent to a model, including dictated text, screen descriptions and attachments, may be logged by the model app under its own retention policy.

## Crash reports

A panic on any thread writes `crash.log` beside the session file: the James version, operating system, thread name, the panic message and location, and a backtrace. Panic messages come from James's code, not from conversation text, though a dependency's message could include a value it was handling. The report is never sent anywhere. James shows it on the next launch with an option to delete it; the person decides whether to paste it into a public issue.

## Releases

Release packages are unsigned unless a maintainer configures certificates (see [INSTALLER.md](docs/INSTALLER.md#signing)); Windows SmartScreen and macOS Gatekeeper warn on first launch. Each release carries SHA-256 checksums, GitHub build-provenance attestations and a `release-metadata.json` that states whether anything was signed. These show a file is the one the workflow built; they do not make it trusted by the operating system. James has not had an independent security review.

## Reporting

No production version is supported. Use [GitHub private vulnerability reporting](https://github.com/topherchris420/james/security/advisories/new) for a private reproduction. Do not post credentials, screenshots, or private research in public issues.
