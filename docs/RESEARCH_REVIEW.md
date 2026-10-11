# Review of existing Korso findings

Reviewed 2026-10-11 against the retained research ledger and full REA evidence export. This was an offline review of existing evidence, not a new Korso analysis or live product assessment. No application code, architecture decision, or reference artifact was changed.

## Assessment

The findings are useful for defining independent James requirements. They establish package identity and contents, and preserve public descriptions of features and data handling. They do not establish the desktop application's actual execution paths, security enforcement, or relative quality.

The report's artifact hashes, eight exported record IDs, inventory counts, extraction count, and integrity-contradiction count match the saved evidence. There is no basis in these records for upgrading the open behavioral questions to verified findings.

The [experimental behavior assessment](BEHAVIOR_EVIDENCE.md) identifies the four demonstrated website steps and the desktop behaviors that remain untested, including the distinction between complete page-step artifacts and W1's incomplete overall event coverage.

## Evidence checked

| Evidence | Review result |
| --- | --- |
| Bundle | 42,671,119 bytes; eight records. Recomputed SHA-256 matches `2c48864becd2c41f448340de3f5913b6d22421e88048c58b30ff9bb96194ce29`. |
| Installer | Recomputed SHA-256 matches `a44f6effcb604ec2f5e8b4296338d0170b17884a402c86cfc8b1ed02792cf8a5`. |
| ASAR | Recomputed SHA-256 matches `20f351cbd359b6e3f85838e7e095e95765aa3ceca7133cdcf90b46fdd347aa22`. |
| I2 / X1 | 2,283 content nodes, 2,453 occurrences, and 2,044 extracted regular files agree with the ledger. These count different things. |
| A1 / V1 | 1,772 relevant files and 17,197,629 text bytes read, but zero parsed JavaScript files and zero AST nodes. Application and semantic coverage are partial. |
| W1 / W2 | Four public-page steps completed; owned browser processes were terminated. These were website captures, not desktop runtime observations. |

Full Evidence IDs are preserved in the [research index](RESEARCH.md#evidence-index). Key review citations:

- W1: `ev_39d99352d7ca40271d7279cf87e04ce754a53b8f21a73d0c1b33149cc11371c4`.
- W2: `ev_aec5eb31baaa4140ed8b7443887ed014472ae8d16a65809f33eee68ade3a502b`.
- A1: `ev_502d73bfc55fef63019806a20d7fd6ae197b2d837d12891f6135be6b94aea51b`.
- V1: `ev_d2670b02b709e7271a9e2c0710b40f29c557915dc4d9135b1aa6ab5873699e60`.
- I2: `ev_20e6bdbc806df3879467a48cae127359f59d7b698c302ff15d1bf356fe387e72`.
- X1: `ev_2e7754016120bd1ccebaa8d33a9f36454bbffd00fce20cb6ae6fb8039a4ffb0c`.

## Findings that can be used

| Finding | Evidence class | Limit |
| --- | --- | --- |
| The captured changelog identifies 0.7.8; the earlier indexed page was stale | Published page content, W2 | Historical capture, not a claim about later releases |
| Main, preload, renderer, native capture, image-processing, updater, input-hook and reporting components are packaged | Observed inventory/extraction, I2/X1 | Presence is not proof of use or enforcement |
| Public pages describe screen selection, visual guidance, voice, dictation, connected accounts, file backups and undo | Published descriptions, W1/W2 | No desktop behavioral test |
| The policy says circling does not restrict capture to the selected area | Published privacy description, W1 | No transmission payload or capture-scope test |
| The provider page names cloud model, action-checking, connector, transcription, infrastructure and reporting providers | Published provider disclosure, W2 | No measured request attribution or backend inspection |

## Gaps and qualifications

1. **Monitor selection is unresolved.** W1's privacy page refers to the primary display; W2's changelog describes the display under the pointer. Neither establishes executable behavior.
2. **Approval policy is unresolved.** W1's per-action description and W2's configurable approval modes should not be collapsed into one inferred rule. W2 also describes a one-time change from Always ask to Auto in release 0.7.2. Verify the migration and effective policy before drawing conclusions. James should make permission changes explicit; that is an original design recommendation.
3. **Six native files have contradictory integrity metadata.** I2/X1 retain expected and observed hashes for three Sharp/libvips files, one capture module, and two input-hook modules. Their cause is unknown. The mismatch does not establish malicious behavior, and the installer's signature does not resolve those contradictions.
4. **Semantic analysis is unavailable in this run.** A1/V1 record the Windows x64 host restriction. Zero IPC, window, or call-flow findings reflect interrupted analysis, not absence of those features.
5. **Network coverage is bounded.** W1 reports requests still unfinished at capture end. The captured website traffic cannot establish all website activity, desktop destinations, or provider retention behavior.
6. **Unknown counts have different scopes.** The bundle's top-level unknown-record list is empty, while V1 reports 1,772 application unknowns and 20,208 semantic unknown entries. The latter are analysis entries, not a count of independent product defects.
7. **Local models, agent coordination, reliability and performance remain unestablished.** Their presence or absence in Korso cannot be inferred from this evidence. No comparative benchmark exists.

## Follow-up priority

First complete bounded static analysis of the same hash-identified package on a supported host, keeping the six integrity contradictions explicit. Then test capture scope, monitor selection, review modes and migration, cancellation, backups/undo, and interrupted side effects against disposable desktop fixtures. Backend/model routing and comparison claims require their own evidence.

The existing James choices of explicit capture previews, immutable review, visible data destinations, and uncertain-outcome reconciliation are reasonable independent requirements. They are not recovered Korso implementation details or demonstrated competitive advantages.
