# Security scope

James v0.3 is an unsigned desktop preview, not an OS sandbox. The desktop now includes local HTTP model/Lab clients, explicit screen capture, Windows voice helpers, and reviewed computer input.

## Network and context

Model and Lab requests go through `james-loopback`, whose only address input is a port. It connects to `127.0.0.1` or `::1` and cannot express another destination; there is no DNS, proxy, TLS or redirect following. Header, line and body sizes are bounded, and connections have connect, idle and total timeouts. Stop shuts the socket down from the UI thread. A local endpoint does not authenticate its process or prove inference stays on-device. Use trusted local servers. No API keys are stored and authenticated model servers are not yet supported. **Find my model server** sends one model-list request to each of ports 1234, 11434 and 8080 on loopback, only when the user presses it, and accepts only a well-formed model list as an answer.

Screen capture occurs only on an explicit click. The preview remains in memory; attachment to a question or action proposal is explicit. Screenshots and raw audio are not saved by James; on macOS and Linux, tools that need a file (screen capture, whisper.cpp) use a fresh `0700` temporary folder created with create-not-reuse semantics and removed immediately. Voice input can only become chat text; it has no path to action approval. Conversation text, including dictation and model descriptions of screen content, is plaintext in the local session file. Model-server logging has its own retention policy.

Lab context comes from the actual versioned discovery endpoint, refreshes while explicitly connected, and enters prompts only when enabled and fresh. Lab and screen contents are untrusted data. Prompt instructions are not a security boundary; model output can only propose an allowlisted action for human review.

## Computer actions

The host accepts one click, allowlisted key, or printable text insertion. No command execution, shell tool, arbitrary process launch, action sequence, or automatic agent loop is exposed to the model. The desktop review displays the exact action, requires a screen captured within 60 seconds, and creates a consumed `ReviewedAction` only after Allow once. Five seconds permit target focus; Stop cancels pending work. Escape works only while James has focus, not globally.

The executor uses `james-core` policy/approval/budget contracts and a create-new, synchronized intent marker before sending input. Existing or unreadable intent markers block input. The marker remains until the user explicitly verifies the target, including after restart or uncertain failure. No interrupted action is retried. This is a conservative single-outstanding-action gate, not a transactional journal or automated outcome verification. Storage/power-loss durability and external application semantics are not proven by unit tests.

The target application is whichever is foreground at dispatch; clicks address the primary monitor. This is not process/window isolation. Focus changes, screen movement, or applications with higher privileges can affect behavior. Display dimensions are rechecked, but unchanged dimensions do not prove unchanged content. Never approve based only on the model's rationale. Input can submit, send, or delete depending on the target. Closing James cancels and waits for native helpers to terminate; already-sent input cannot be undone. Killing the process externally is not cancellation proof.

Windows helpers run fixed embedded scripts through the system PowerShell path; user/model strings are JSON stdin, never interpolated executable commands. Native input requires platform API interop inside fixed helpers; the Rust workspace still forbids unsafe Rust. These helpers are not sandboxed. On macOS and Linux, helpers are system tools resolved only from fixed system directories (never the search path) and run without a shell; a whisper.cpp program or model path entered in Settings must be an absolute path to an existing file and is trusted as the user's choice. Clicks and typing outside Windows return unsupported errors.

## Crash reports

A panic on any thread writes `crash.log` beside the session file: the James version, operating system, thread name, the panic message and location, and a backtrace. Panic messages come from James's code, not from conversation text, though a dependency's message could include a value it was handling. The report is never sent anywhere. James shows it on the next launch with an option to delete it; a user decides whether to paste it into a public issue.

## Reporting

No production version is supported. Use [GitHub private vulnerability reporting](https://github.com/topherchris420/james/security/advisories/new) for a private reproduction. Do not post credentials, screenshots, or private research in public issues.
