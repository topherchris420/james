# Korso observations and assumptions

Classified 2026-10-11 using the retained REA evidence. No new target execution or feature testing was performed. This classification applies to the recorded artifact and browser sessions, not later releases.

## Evidence classes

| Class | Meaning |
| --- | --- |
| Observed behavior | A declared experiment exercised the target and recorded the result, within the stated coverage |
| Observed artifact fact | Inspected bytes, metadata or package structure; does not establish execution |
| Published claim | Korso describes a feature or policy; the description was observed, but the described behavior was not tested |
| Inference or assumption | An interpretation or working hypothesis beyond what the evidence establishes; must remain provisional |
| Unknown | The retained evidence cannot resolve the question |
| James design choice | An original requirement or architecture decision; neither a recovered Korso implementation nor proof of James's finished functionality |

## What is observed

| Finding | Class and evidence | Supported conclusion and boundary |
| --- | --- | --- |
| Homepage, privacy policy, changelog and subprocessor pages loaded and exposed rendered content | Observed behavior, W1/W2 | Four public-page navigation steps completed. No authenticated assistant interaction or desktop feature was exercised. W1 overall event coverage is incomplete. |
| The examined installer and package identify version 0.7.8 | Observed artifact fact, P1/X1 plus inert metadata reads | Identifies this artifact; does not establish successful installation, executable behavior or the version currently available. |
| The package includes main/preload/renderer entries and capture, input-hook, image-processing, updater and reporting components | Observed artifact fact, I2/X1 | These components are present. Their active use, permissions and runtime interactions remain unestablished. |
| Windows reported a valid installer signature naming Korso, Inc. | Observed artifact fact, auxiliary Windows verification | Records the signature verification result. Does not establish behavioral safety or resolve the six native-file integrity contradictions. |
| Six native-file hashes disagree with declared integrity metadata | Observed artifact fact, I2/X1 | Contradictions exist in this artifact; their cause is unknown. They do not establish malicious behavior. |
| JavaScript semantic analysis remained incomplete | Observed analysis outcome, A1/V1 | Zero parsed JavaScript files and zero AST nodes reflect the host restriction. Zero resulting feature findings do not establish feature absence. |

The [experimental behavior assessment](BEHAVIOR_EVIDENCE.md) contains the step-level results and limitations. No Korso desktop AI functionality was experimentally observed.

## Published functionality versus assumptions

| Topic | Published claim, W1/W2 | Assumption that must not become a finding | Current status |
| --- | --- | --- | --- |
| Visual context and guidance | Pointing/circling screen content, answers and annotations | Selecting a region means only that region is captured or transmitted | Unverified; policy says circling does not exclude the rest of the display |
| Capture timing | Capture accompanies a question; policy describes no continuous screen recording | Capture occurs only at that time in every execution path | Runtime timing and enforcement unknown |
| Monitor selection | Privacy page refers to the primary display; changelog refers to the display under the pointer | Either description establishes the executable's current rule | Unresolved discrepancy |
| Voice and dictation | Voice input, dictation and speech features are described | Transcription accuracy, playback quality or safe dictation behavior is established | Unverified |
| Desktop actions | Clicking, typing, opening applications/links and scripts are described | Packaged input hooks prove these actions execute correctly | Unverified |
| Approval policy | Per-action approval with exceptions; configurable modes and a one-time default-mode change are described | Every action requires approval, or one mode description defines all releases and categories | Effective rules and migration unverified |
| Waiting for typing | Changelog describes waiting until typing stops before script clicks/typing | The wait is reliably enforced under concurrent input | Unverified |
| Backups and undo | Changelog describes backups before file changes and undo | Every effect is reversible, backup-complete or recoverable after a crash | Unverified; cancellation and crash recovery also unknown |
| Cloud models and integrations | Subprocessor page lists named providers and roles | Every listed provider is active in every session, or all requests follow the disclosed route | Actual routing and account-action behavior unknown |
| Privacy and history | Policy describes server-held conversation text, image/audio handling and local encrypted sign-in credentials | Capturing policy text proves storage, deletion, encryption or third-party retention behavior | Enforcement and provider-side behavior unknown |

The observation “the page states X” is supported. The conclusion “the desktop app does X” is not supported by these page captures alone.

## Other inferences and unknowns

- The package layout is consistent with an Electron application. This structural interpretation does not recover its executed architecture, IPC contracts, prompts or security boundaries.
- Native capture and input-hook components suggest implementation roles. Their names and presence do not establish exact call paths or activation conditions.
- Local-model support, agent coordination, sandboxing, failure handling, latency, resource use and comparative reliability remain unknown. Lack of evidence is not evidence of absence.
- A valid outer signature and contradictory per-file metadata are separate observations. Neither resolves the cause of the other.
- James superiority is an untested aspiration. No matched comparative benchmark exists.

## James decisions stay independent

Explicit capture previews, visible data destinations, immutable approval requests, local-model adapters, bounded agent coordination and uncertain-outcome reconciliation are James design choices. They must be justified and tested on their own. They are not established Korso internals, and a proposed James component is not an implemented capability.

James's current core contract tests establish behavior of that core under its test fixtures. They do not establish desktop integration, real tool effects, provider behavior or comparative advantage.

## Sources and promotion rule

Evidence aliases resolve to full REA Evidence IDs in [RESEARCH.md](RESEARCH.md#evidence-index). Bundle identity and verified artifact hashes are recorded in [RESEARCH_REVIEW.md](RESEARCH_REVIEW.md). The assessment uses the existing bundle with SHA-256 `2c48864becd2c41f448340de3f5913b6d22421e88048c58b30ff9bb96194ce29`.

Promote a behavioral claim to observed only after an experiment exercises that behavior and records the target version, inputs, outputs or effects, environment, Evidence ID and coverage limits. Preserve contradictory descriptions until evidence resolves them. A static package finding can establish an artifact fact without being promoted to runtime behavior.
