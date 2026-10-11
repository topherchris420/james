# Experimentally demonstrated Korso behaviors

Assessed 2026-10-11 from the existing REA evidence export. This assessment adds no new experiments.

## Conclusion

No Korso desktop AI behavior has been experimentally demonstrated in the retained evidence. The installer and desktop application were not executed. Two controlled browser scenarios demonstrated four public-page loads and rendered content. Reading a feature description or observing a packaged component does not demonstrate the described feature executing.

## Demonstrated website behavior

| Experiment step | Observed result | Evidence | Boundary |
| --- | --- | --- | --- |
| Load `https://korsoai.com/` | Navigation completed; final URL and accessibility content captured | W1, `scenario_start` | Establishes the public page loaded in this browser session; no assistant interaction was exercised |
| Navigate to `https://korsoai.com/privacy` | Navigation completed; final URL, accessibility content and DOM captured | W1, `privacy-policy` | Establishes the policy was rendered; does not validate capture scope, transmission, retention or consent enforcement |
| Load `https://korsoai.com/changelog` | Navigation completed; final URL and accessibility content captured | W2, `scenario_start` | Establishes release descriptions were rendered; does not validate their desktop behavior |
| Navigate to `https://korsoai.com/subprocessors` | Navigation completed; final URL and accessibility content captured | W2, `subprocessors` | Establishes provider disclosures were rendered; does not validate actual routing or provider use |

Both scenarios report authority `controlled-replay` and confidence `observed`. All four navigation steps report `completed`, with complete requested step artifacts. They used launched Chromium 154.0.8037.99 with no sign-in. The owned browser processes were terminated after capture; that is REA cleanup behavior, not a Korso feature.

W1's overall completeness is **incomplete**, with `events` missing from complete coverage and some requests unfinished at capture end. Its complete page-step artifacts do not imply complete network coverage. W2 reports complete coverage for its narrower requested scenario. Neither scenario captured desktop requests or exercised an authenticated assistant session. No screenshot was requested for these steps; rendering is evidenced by accessibility content and, for the privacy page, DOM.

## Desktop behaviors not experimentally established

| Behavior or claim | Existing evidence class | Experiment needed to establish it |
| --- | --- | --- |
| Screenshot capture, selected region handling and monitor selection | Public descriptions; packaged capture component | Use distinguishable disposable displays and inspect actual capture/transmission payloads |
| Contextual answers, visual annotations and cursor-adjacent guidance | Public descriptions | Ask a question against a controlled desktop fixture and record the response and visible guidance |
| Voice input, dictation and speech output | Public descriptions; provider disclosure | Run controlled audio fixtures and observe recognized text, inserted text and playback |
| Clicking, typing, opening apps/links and script execution | Public descriptions; packaged input-hook components | Execute bounded actions in disposable fixtures and record requested versus actual effects |
| Approval modes, per-action review and default-mode migration | Public descriptions with unresolved qualifications | Test each mode, action category and upgrade path; record effective policy and dispatch behavior |
| Waiting while the user types | Changelog description | Interleave controlled typing with a pending action and measure dispatch timing |
| Cancellation, backups, undo and crash recovery | Public descriptions where available; open research questions | Interrupt disposable actions and verify resulting state, restoration and recovery |
| Connected-account actions and external side effects | Public descriptions; provider disclosure | Use a test account and verify authorization, dispatch, effect and replay behavior |
| Model/provider routing, privacy promises and retention | Public disclosures | Observe actual requests and separately verify the relevant storage/deletion behavior; page text alone is insufficient |
| Local-model support and agent coordination | Unestablished | Exercise an identified model/runtime or coordinated task and record execution evidence |
| Reliability, latency, resource use or superiority to James | No benchmark results | Run matched, repeatable tasks with versions, environments, failures and uncertainty recorded |

“Not experimentally established” does not mean absent or defective. It means the retained experiments do not answer the question.

## Static observations are separate

Installer identity, signature checks, package metadata, file inventory and extraction were verified. These establish properties of the examined artifact, not desktop execution. JavaScript semantic analysis remained partial with zero parsed JavaScript files and zero AST nodes; its lack of runtime or call-flow findings cannot establish feature absence. Six native-file integrity contradictions remain unresolved.

The James core's own contract tests are evidence about James only and do not validate Korso behavior.

## Evidence provenance

- W1: `ev_39d99352d7ca40271d7279cf87e04ce754a53b8f21a73d0c1b33149cc11371c4`, `capture_browser_scenario`.
- W2: `ev_aec5eb31baaa4140ed8b7443887ed014472ae8d16a65809f33eee68ade3a502b`, `capture_browser_scenario`.
- Full artifact and analysis Evidence IDs are in the [research index](RESEARCH.md#evidence-index); the [review](RESEARCH_REVIEW.md) records verified hashes and coverage limits.
- Retained bundle: `C:\Users\chris\.cache\james-korso\2026-10-11\rea-evidence.json`, SHA-256 `2c48864becd2c41f448340de3f5913b6d22421e88048c58b30ff9bb96194ce29`.

This is a historical assessment of these records, not a claim about the current website or a later Korso release.
