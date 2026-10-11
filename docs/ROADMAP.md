# James implementation roadmap

Status: Phase 1 executable core with in-memory supervised dispatch. This roadmap sequences original implementation; it makes no competitive outcome claim.

| Milestone | Deliverable | Exit gate |
| --- | --- | --- |
| Phase 1 — research and architecture | Evidence ledger, ADRs, requirements, pure Rust authorization/lifecycle contracts, supervisor and journal-callback gate, tests, comparison protocol, publication files | Local checks pass; known gaps named; no reference code/assets in repository |
| Phase 2 — working desktop chat | eframe conversation/provider/timeline UI; Tokio supervision; one LM Studio or Ollama adapter; cancellable streaming; local SQLite history | Real UI → runtime → provider → store flow tested; malformed stream/disconnect/cancel fixtures; no implicit cloud fallback |
| Phase 3 — controlled tools and recovery | Read-only workspace tool, typed registry, concrete reviews, path enforcement, durable intent/outcome journal; later staged writes | Actual tool dispatch cannot bypass policy; crash-boundary/revocation/path-race tests; unknown outcome reconciliation |
| Phase 4 — providers and extensions | Ollama/LM Studio/llama.cpp compatibility profiles; capability discovery; versioned MCP client; supervised extension processes | Captured protocol fixtures; bounded messages/timeouts; install trust disclosure; no claimed sandbox without enforcement |
| Phase 5 — coordinated agents | Bounded task DAG, planner/worker/reviewer roles, scoped context, shared run budgets, artifact ownership | Conflict, cancellation, and failure tests; single-agent comparison demonstrates whether coordination helps |
| Phase 6 — desktop context and voice | Platform-specific capture/automation, explicit previews, visual guidance, STT/TTS/dictation | Scope and microphone tests; actual multi-monitor/scaling/accessibility fixtures; visible destinations and retention |
| Phase 7 — public desktop release | Supported-platform builds, signed packages and updates, docs, dependency/license inventory, diagnostics, benchmarks | End-to-end/fault/accessibility checks; maintainers configured; publish only supported claims and measured comparisons |

## Phase 2 implementation order

1. Create a minimal Rust window and typed command/event channels. Keep I/O off the UI thread.
2. Build the provider adapter against recorded synthetic responses before touching a user's server. Test fragmented streams, malformed frames, context errors, timeouts, and cancellation.
3. Add explicit provider settings and a connection test. Do not load/unload models or change the user's inference service silently.
4. Complete a real typed conversation with streaming and a verifiable terminal result.
5. Add SQLite migrations and history. Restart during a stream and show incomplete work honestly.
6. Verify keyboard/assistive-technology review, scaling, long conversations, and UI responsiveness. Record actual platform support.

Phase 1 tests establish contracts for later integration. They do not remove Phase 2–7 exit gates or constitute a functioning desktop product.
