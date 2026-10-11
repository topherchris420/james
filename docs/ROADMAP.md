# James implementation roadmap

Status: Phase 1 core complete as a prototype; Phase 2 **in progress** with a native desktop window, localhost LM Studio model discovery/chat, local plaintext history, and a Windows workflow artifact. Streaming, durable history, real Lab integration and end-to-end authority remain unimplemented.

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

1. **Implemented first slice:** minimal eframe window and a worker-to-UI channel, with model HTTP I/O off the UI thread.
2. Build the provider adapter against recorded synthetic responses before touching a user's server. Test fragmented streams, malformed frames, context errors, timeouts, and cancellation.
3. **Partially implemented:** explicit local port and model settings, model discovery and mock HTTP tests. Do not load/unload models or change inference services silently.
4. **Partially implemented:** typed text conversation and terminal HTTP result exist. Add incremental streaming and cancellation.
5. **Partial local history:** plaintext JSON session file exists. Replace it with SQLite migrations and crash-aware incomplete-run tracking.
6. Verify keyboard/assistive-technology review, scaling, long conversations, and UI responsiveness. Record actual platform support.

The new Windows portable build is a developer preview, not a signed release. Core contract tests and GUI HTTP mocks do not remove Phase 2–7 exit gates. The desktop currently cannot authorize or execute tools.
