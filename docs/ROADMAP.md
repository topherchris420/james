# James implementation roadmap

Status (0.4.0, developer preview): a native desktop that streams replies from LM Studio, Ollama or llama.cpp through a loopback-only transport, with guided first-run setup, model capability detection, labelled cloud models, plain-language errors, Markdown, themes and accessibility settings, privacy controls, screen questions and Guide me, voice, a read-only R.A.I.N. Lab view, and reviewed single-action input on Windows behind a durable intent record and a target-window check. Real-device validation on Windows and macOS, screen-reader testing, signed releases, automated outcome verification and transactional history remain open; see [END_USER_READINESS.md](END_USER_READINESS.md).

| Milestone | Deliverable | Exit gate | State in 0.4.0 |
| --- | --- | --- | --- |
| Phase 1 — research and architecture | Evidence ledger, ADRs, requirements, pure Rust authorization/lifecycle contracts, supervisor and journal-callback gate, tests, comparison protocol, publication files | Local checks pass; known gaps named; no reference code/assets in repository | Done |
| Phase 2 — working desktop chat | eframe conversation/provider/timeline UI; Tokio supervision; one LM Studio or Ollama adapter; cancellable streaming; local SQLite history | Real UI → runtime → provider → store flow tested; malformed stream/disconnect/cancel fixtures; no implicit cloud fallback | Mostly done: three model apps, cancellable streaming and real-server runs, on worker threads rather than Tokio. History is a JSON file, not SQLite |
| Phase 3 — controlled tools and recovery | Read-only workspace tool, typed registry, concrete reviews, path enforcement, durable intent/outcome journal; later staged writes | Actual tool dispatch cannot bypass policy; crash-boundary/revocation/path-race tests; unknown outcome reconciliation | Not started for tools. The desktop input path has a single-record intent file with human reconciliation |
| Phase 4 — providers and extensions | Ollama/LM Studio/llama.cpp compatibility profiles; capability discovery; versioned MCP client; supervised extension processes | Captured protocol fixtures; bounded messages/timeouts; install trust disclosure; no claimed sandbox without enforcement | Partly done: capability discovery and fixtures recorded from real Ollama and llama.cpp. No MCP or extensions |
| Phase 5 — coordinated agents | Bounded task DAG, planner/worker/reviewer roles, scoped context, shared run budgets, artifact ownership | Conflict, cancellation, and failure tests; single-agent comparison demonstrates whether coordination helps | Not started |
| Phase 6 — desktop context and voice | Platform-specific capture/automation, explicit previews, visual guidance, STT/TTS/dictation | Scope and microphone tests; actual multi-monitor/scaling/accessibility fixtures; visible destinations and retention | Partly done: capture, Guide me and voice on all three systems; input on Windows only; one monitor; microphones untested |
| Phase 7 — public desktop release | Supported-platform builds, signed packages and updates, docs, dependency/license inventory, diagnostics, benchmarks | End-to-end/fault/accessibility checks; maintainers configured; publish only supported claims and measured comparisons | Partly done: packages, checksums, provenance attestations, optional signing, docs and an evidence-scored readiness gate. Third-party license notices in every package. No certificates or real-device runs |

## Next steps

In the order that most improves what people can rely on:

1. Run the [acceptance scenarios](ACCEPTANCE_TESTS.md) on real Windows 11 and macOS machines, including one real reviewed input and a forced quit during the countdown.
2. Test LM Studio, and the app with NVDA, VoiceOver and Orca.
3. Configure signing certificates and publish a signed, attested release.
4. Commission an independent security review of `james-loopback`, the review and input path, the intent record and the Windows helpers.
5. Replace the JSON session with SQLite and crash-aware run tracking (Phase 2), then begin Phase 3 with one read-only workspace tool.

The desktop authorizes one reviewed Windows input at a time; broader tools, input on macOS and Linux, and multi-step automation each need their own reviewed design before they are built.
