# Korso research ledger

Investigation date: 2026-10-11. Scope: public product pages and the publicly distributed Windows x64 installer. No Korso desktop runtime, signed-in account, private service, or backend source was inspected. Findings inform original James requirements; they do not specify a clone.

The [existing-evidence review](RESEARCH_REVIEW.md) cross-checks this ledger against the saved bundle and records additional qualifications about approval-mode migration and bounded network coverage.

The [observations and assumptions classification](OBSERVATIONS_AND_ASSUMPTIONS.md) separates demonstrated behavior, artifact facts, published claims, provisional interpretations and James design choices. The [experimental behavior assessment](BEHAVIOR_EVIDENCE.md) lists the four demonstrated website steps; no desktop AI behavior was tested.

## Evidence rules

**Observed artifact** means bytes, metadata, or structure were inspected. **Published claim** means a page describes behavior, without proving that behavior runs. **Inference** means the analyst interpreted evidence. **Unknown** means the investigation could not establish a result. Website browser captures observed the website, not the desktop assistant.

REA was used through the connected MCP server. Public browser scenarios used an isolated, owned headless Chrome process with empty storage and no sign-in. Browser cleanup was reported as complete. The installer was downloaded from the link published on [Korso's site](https://korsoai.com/), verified with Windows Authenticode, and statically unpacked with 7-Zip 26.04. Korso's installer, app, native modules, and recovered JavaScript were never executed.

Raw evidence and reference files are kept outside James, under the local research cache `.cache/james-korso/2026-10-11`. The complete `rea-evidence.json` export is 42,671,119 bytes and contains eight records. Its session-level unknown-record count is zero; this does **not** mean analysis coverage has no unknowns. The interrupted JavaScript analysis separately reports substantial unknown coverage.

The exported bundle SHA-256 is `2c48864becd2c41f448340de3f5913b6d22421e88048c58b30ff9bb96194ce29`. The active REA archive session was closed after export.

## Target identity

| Item | Identity |
| --- | --- |
| Public distribution | `https://updates.korsoai.com/desktop/win32-x64/Korso-windows-x64-setup.exe` |
| Downloaded bytes | 143,918,224 |
| Installer SHA-256 | `a44f6effcb604ec2f5e8b4296338d0170b17884a402c86cfc8b1ed02792cf8a5` |
| Authenticode | Windows reported a valid signature; signer subject names Korso, Inc. |
| Installer container | PE32 x86 NSIS stub containing a Windows x64 application archive; stub architecture is not application architecture |
| Installer version metadata | 0.7.8 |
| Application archive | `resources/app.asar`, 33,417,124 bytes |
| ASAR SHA-256 | `20f351cbd359b6e3f85838e7e095e95765aa3ceca7133cdcf90b46fdd347aa22` |
| Root package manifest | `@korso/desktop`, version 0.7.8, main entry `out/main/index.js` |
| Package manifest SHA-256 | `7e04c843ca0caff3cf88290adf58f0c8bef0dd86db6b3d60457e48bb7fbea0dd` |

The live [changelog](https://korsoai.com/changelog) identified 0.7.8 on October 11, matching package metadata. The search-indexed copy initially ended at 0.7.0. Similarly, the indexed privacy page showed September 30 while the directly captured page showed October 5. Prefer the retained direct capture for this investigation; document versions rather than silently merging them.

## Evidence index

| Alias | REA Evidence ID | Coverage |
| --- | --- | --- |
| W1 | `ev_39d99352d7ca40271d7279cf87e04ce754a53b8f21a73d0c1b33149cc11371c4` | Homepage then privacy page; accessibility, final DOM, attach-window website network metadata |
| W2 | `ev_aec5eb31baaa4140ed8b7443887ed014472ae8d16a65809f33eee68ade3a502b` | Live changelog then subprocessors page; accessibility and URLs |
| P1 | `ev_2480bdc078955e78131d5ef503ba43874c62c0923d6e969abbd77741cb8b46c5` | Portable PE resource tree and installer identity; no app execution |
| A1 | `ev_502d73bfc55fef63019806a20d7fd6ae197b2d837d12891f6135be6b94aea51b` | Interrupted application analysis; inventory retained, no JavaScript AST analysis completed |
| V1 | `ev_d2670b02b709e7271a9e2c0710b40f29c557915dc4d9135b1aa6ab5873699e60` | Summary projection of A1, preserving coverage and integrity limitations |
| I1 | `ev_890fa8a6b7190db68b7ee16f97f7f70260e7f5e0338e8cef4a02e2638549ab75` | Artifact inspection; 2,283 content nodes and 2,453 occurrences |
| I2 | `ev_20e6bdbc806df3879467a48cae127359f59d7b698c302ff15d1bf356fe387e72` | Inventory nested in I1; content hashes and occurrence paths |
| X1 | `ev_2e7754016120bd1ccebaa8d33a9f36454bbffd00fce20cb6ae6fb8039a4ffb0c` | Safe ASAR extraction; 2,044 regular files; six integrity contradictions retained |

P1 identifies resources and hashes; 7-Zip supplied the NSIS/version metadata, and Windows supplied signature validation. Package manifest fields were read as inert JSON from X1's hash-identified extraction. These auxiliary observations are not invented REA semantic results.

## Findings and original design implications

“Published only; behavior unverified” means the public description is available, but execution and enforcement have not been established. James choices in the final column are original requirements, not recovered Korso functionality or claims of completed James integration.

| Question | Finding / evidence class | Status | James design choice |
| --- | --- | --- | --- |
| How does a user supply context? | Site describes pointing/circling content and answering with screen annotations. Published claim, W1 / [homepage](https://korsoai.com/). | Published only; behavior unverified | Explicit region/window/display selection with preview; visual guidance may come later. |
| Does selection restrict captured data? | Policy states that circling does not exclude the rest of the display; capture accompanies a question. Published claim, W1 / [privacy](https://korsoai.com/privacy). | Published only; behavior unverified | Capture and transmission scope must match the preview. Record destination and retention; do not silently expand a crop. |
| Voice and dictation? | Pages describe voice interaction and dictation. Changelog describes new voices, script checks, and waiting for typing to stop. Published claim, W1/W2 / [changelog](https://korsoai.com/changelog). | Published only; behavior unverified | Separate speech input, spoken output, dictation, and actions; visible microphone state and cancellation. |
| Actions and undo? | Changelog describes review modes, backups before file changes, and undo. Policy describes per-action approval with an opening-link/app exception. Published claims, W1/W2. Actual rules unresolved. | Published only; behavior unverified | Concrete approval contracts, immutable arguments, staged file changes, honest reconciliation; no blanket “never ask” mode initially. |
| Cloud and connected accounts? | Provider page names OpenAI, TypeSafe AI, Composio, Soniox, AssemblyAI, Google Cloud/Firebase, PostHog, and Sentry for their respective roles. Published claims, W2 / [subprocessors](https://korsoai.com/subprocessors). | Published only; behavior unverified | Independent provider adapters, explicit data destinations, optional connections, no required account for on-device usage. |
| History and retention? | Policy describes server-held conversation text, no server storage of captured images/audio, and local encrypted sign-in credentials. Published claim, W1. Third-party processing remains separate. | Published only; behavior unverified | Local retention controls, export/deletion, credentials outside journals; privacy guarantees require integration tests. |
| Shipped desktop packaging? | Hash-identified ASAR contains main/preload/renderer entries, capture native module, image library, updater, input-hook and error-reporting dependencies. Observed artifact, I1/I2/X1. | Answered for this artifact | Rust-rendered James UI and typed host boundaries. Package presence does not prove runtime execution. |
| IPC, prompts, model routing, security enforcement? | REA's source analyzer did not run on this host; no completed AST or call-flow evidence. A1/V1. | Unresolved | Do not imitate inferred internals. Implement and verify James's own contracts. |
| Local models or multi-agent support in Korso? | No runtime or complete semantic investigation established either capability or absence. | Unresolved | Treat both as James goals, not a basis for superiority claims. |
| Relative performance/reliability? | No matched desktop benchmark was run. | Unresolved | Use [BENCHMARKS.md](BENCHMARKS.md) before comparative claims. |

Monitor selection also needs verification: the policy refers to the primary display, while the changelog describes the display under the pointer. Preserve the discrepancy; neither page proves current executable behavior.

## Integrity and coverage limitations

Default ASAR analysis rejected a companion native-file integrity mismatch. A second request explicitly used `integrity_policy: "record-and-continue"`, preserving observed bytes as untrusted. Six files disagree with declared hashes: three Sharp/libvips files, the capture native module, and two input-hook modules. The evidence records both expected and observed digests. Their cause is unknown; a valid installer signature does not reconcile per-file metadata contradictions.

The connected JavaScript analyzer then reported: “REA Windows native controls require a Windows x64 host.” This machine's native host is Windows ARM. A1 retained inventory and read 17,197,629 text bytes across 1,772 relevant files, but parsed **zero** JavaScript files and visited **zero** AST nodes. Its zero IPC/window findings are missing analysis, not evidence that those constructs are absent. Portable artifact inventory, extraction, PE resources, and browser scenarios remained available.

This Phase 1 report is a bounded evidence review and architecture input. A deep behavioral reconstruction of Korso remains incomplete. To resolve the open questions, use a supported analysis host and disposable, explicitly configured desktop fixtures; verify each feature with runtime evidence rather than extending static conclusions.

## Reproduction outline

1. Download the public installer to a directory outside the James repository. Record time, byte length, SHA-256, and signer. A changing latest-download URL may produce a different artifact; do not silently reuse this report's identity.
2. List and extract its nested application archive using a static extractor. Preserve `app.asar.unpacked` alongside `app.asar`.
3. Call REA `inspect_pe_resources` with the installer path and `max_file_bytes: 160000000`.
4. Call `analyze_javascript_application` with the ASAR path, `format: "asar"`, and `detail: "summary"`. Preserve any integrity rejection. If electing to continue, declare `integrity_policy: "record-and-continue"` and retain contradictions.
5. Reuse the returned `parent_evidence_id` for `inspect_analysis_view`; never interpret an incomplete overview's zeros as absence. On this host the partial parent was A1.
6. Open the ASAR with `open_binary`, then use `inspect_artifact` and `extract_artifact` with the chosen integrity policy. Read only selected hash-identified inert metadata.
7. Capture the public pages using declared `capture_browser_scenario` navigations, empty storage, and no sign-in. Export the full evidence bundle before `close_binary` clears retained references.

Use the actual connected schemas and record software versions. These are operation descriptions, not a claim that all REA CLI argument forms or other host combinations were tested.
