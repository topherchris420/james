# James requirements

Status: original Phase 1 specification, 2026-10-11. Proposed acceptance criteria are not implemented features. Trace reference-derived motivation through [RESEARCH.md](RESEARCH.md); architecture decisions through [ARCHITECTURE.md](ARCHITECTURE.md).

| ID | Requirement | Acceptance evidence | Current state |
| --- | --- | --- | --- |
| J01 | Rust application with a Rust-rendered desktop UI | Build and operate an eframe conversation/review/timeline flow on supported platforms | Core only; UI proposed |
| J02 | Explicit context capture | Preview contains exactly the region/window/display sent; hidden fixtures never leave that scope | Proposed |
| J03 | Useful local-model operation | Complete chat offline with configured on-device server; show model/backend and capabilities | Proposed |
| J04 | Honest provider locality | Distinguish endpoint location from inference locality; no silent provider/cloud fallback | Proposed |
| J05 | Immutable action review | Changed run, tool, arguments, scope, destination, or action generation cannot use prior consent | In-memory contract tested |
| J06 | Immediate revocation and expiry | Reject dispatch after expiry, changed policy, denial, or pre-dispatch cancellation | In-memory contract tested |
| J07 | Bounded work | Enforce run-owned dispatch/token/deadline ceilings; later add model-call/depth/worker limits | Shared in-memory run budget implemented; host limits proposed |
| J08 | Honest recovery | Restart invalidates consent; unresolved side effects cannot auto-retry; durable journal reconstructs state | In-memory transitions and intent callback implemented; durability proposed |
| J09 | Supervised coordination | Dependency-aware worker graph with scoped context, cancellation, conflict resolution, and provenance | Proposed |
| J10 | Extensibility | Versioned provider/tool APIs and MCP fixtures; arbitrary subprocess trust visible | Proposed |
| J11 | Explainable execution | Show concrete action, destination, permission, observed result, errors, and uncertainty | Proposed UI |
| J12 | User-owned history | Local retention, export/delete, redacted diagnostics, no unsolicited analytics upload | Proposed |
| J13 | Voice under user control | Separate input/output/dictation permissions; visible recording and bounded follow-up window | Proposed |
| J14 | Accessible, responsive desktop | Keyboard review, screen-reader inspection, scaling/high contrast, bounded UI latency under inference load | Proposed; measure before release |
| J15 | Reproducible evidence | Hash-identified reference artifacts, evidence classes, explicit unknowns and repeatable checks | Phase 1 documents and tests |
| J16 | Comparative claims require results | Frozen task corpus, matched configs, failures retained, measured outcomes with uncertainty | Protocol prepared; no results |

## Original interaction design

Use a conventional James conversation window with an expandable execution timeline. A context button opens a region/window/file picker; selected material appears in a preview with its intended provider destination. Typing a question does not automatically capture the display. A review card shows the exact operation, resource scope, arguments, destination, and lifetime before dispatch. Editing the operation creates a new review.

The timeline shows queued, waiting for review, running, cancellation requested, completed, failed, and uncertain operations. “Stop” requests cancellation; it does not falsely report a committed side effect as undone. A separate recovery panel identifies unresolved operations after restart. Provider settings expose model capabilities, endpoint location, inference locality, and explicit cloud opt-in.

These are original interface choices; no Korso tab, artwork, pet, tour, typography, sounds, or screen arrangement is reproduced.

## Delivery boundary

Prioritize Windows for the first real desktop slice, while keeping the Rust core portable. Windows x64, native Windows ARM, macOS, and Linux support each require their own build and runtime evidence. Configured CI targets are not proof that they have passed. Screen capture, accessibility automation, credentials, and sandboxing need platform adapters and platform tests.

The first desktop MVP is typed chat with one explicit local provider, cancellable streaming, local history, and a read-only workspace tool through the real policy path. Add writes only after intent journaling, path/resource enforcement, staged changes, and recovery verification exist. Add voice, visual guidance, arbitrary extensions, and agent coordination incrementally.
