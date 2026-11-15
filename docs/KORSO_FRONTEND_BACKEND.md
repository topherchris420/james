# Korso evidence and original James frontend/backend improvements

Assessed 14 November 2026. This pass modifies the actual Rust desktop frontend
and its local state handling. It does not replace James with a web mockup or
reproduce Korso's hosted services. The prior [reconstruction report](KORSO_REPLICATION.md)
contains the broader package analysis and its limitations.

## Artifact identity and scope

The repository's existing reconstruction report identifies the reference ASAR
at `C:\Users\chris\.cache\james-korso\2026-10-11\application\resources\app.asar`.
Its SHA-256 was independently checked again:

```text
20f351cbd359b6e3f85838e7e095e95765aa3ceca7133cdcf90b46fdd347aa22
```

The installed `Korso-Beta/resources/app.asar` has the same SHA-256. Packaged
JavaScript was inspected as inert text from the existing extraction recorded in
the earlier report. References below are paths and line numbers within that
extraction, not James source or an upstream source checkout.

REA's current inventory summary is
`ev_f397e7171e1c3e4b11b9eee08ddaf512e48eccd6acca285e035c025362f2e40e`,
derived from parent
`ev_c8a680c9e0a29c0f68c2d0c9368a6b5f31bf90f3652940255a27362990bea232`.
The manifest is
`agm_9ebc1ad5c4c842dc2dca124612605cd04024e692c4ff0c1c8c2b458eb7e929b9`.
It inventories 2,045 files and retains six native-file integrity contradictions.
Those contradictions remain unresolved; they neither establish maliciousness
nor disappear when JavaScript is readable. This pass does not claim complete
AST coverage, native-code analysis, or product parity.

The current guide analysis is retained as
`ev_7aa66e67bf7be0f2375aa1ced3ab6c454efcfea0e8b40065e98268da6e5d5cd3`.
The feature references below combine that structural REA evidence with manual
inspection of selected extracted files. Neither establishes runtime reachability
or complete coverage of the guide, its native integrations, or its services.

The coding model also invoked REA for the attachment pass. Its summary view is
`ev_22e9eabf20d0eda333d25e75802af33877221ac3c1a619b5c3af047748d23e11`,
from parent
`ev_d4fd5d4f9064d05f407961c34c62dd40330d32efd5ae14c1d99c584630c0d7a4`.
The selected-source graph reached its semantic node/unknown retention limits;
1,770 inventoried files were outside analysis scope. A complete summary view is
not a claim of complete graph or application coverage.

## Feature evidence

**SAW (static)** means the stated code or contract was inspected in the shipped
artifact. It does not mean its full user flow ran. **SAW (runtime)** means the
visible result was observed in the installed app. **INFERRED** remains unverified.

| Feature | How it works in the inspected artifact | Evidence | Classification |
| --- | --- | --- | --- |
| History navigation | A row carries a title, time and current-selection state. The current row returns to chat; another row opens its conversation. History groups rows and confirms deletion. | `out/renderer/assets/panel-DHxMl8DT.js:4027–4094,4112–4229`; re-read row code in this pass. | SAW (static) |
| History lifecycle | The coordinator manages list/read/create/delete operations with cancellation, generation checks and an operation deadline. | `out/main/primary-wShsXXbd.js:24131–24340,39506–39535`; retained earlier static analysis documented in the reconstruction report. | SAW (static; earlier retained evidence) |
| Individual mark dismissal | A closer layer offers controls for ordinary unpinned marks, performs hit testing, and excludes pinned/tour marks. | `out/renderer/assets/overlay-BdilAO27.js:715–848`; closer logic re-read in this pass. | SAW (static) |
| Validated overlay messages | The preload checks message keys, shape kinds, point data and bounded batches before forwarding drawing operations. | `out/preload/overlay.js:78–164`; re-read in this pass. | SAW (static) |
| Pending files | Multiple selected files append in order. Per-file summaries support removal; invalid candidates preserve existing files and accepted siblings. | `out/renderer/assets/panel-DHxMl8DT.js:2636–2694`; `out/main/primary-wShsXXbd.js:23823–23914,29912–29923`; selected-source REA attachment summary above plus manual static inspection. | SAW (static) |
| Request file lifecycle | Frozen pending contents move into the request, interrupted requests can restore them, and request media is released afterward. Hosted parsing and retention are unverified. | `out/main/primary-wShsXXbd.js:28465–28485,28678–28682`; selected-source REA attachment summary above plus manual static inspection. | SAW (static) |
| Structured guide plan | A plan carries a goal, a target window, completed titles and up to 12 steps. Each step has a title, instruction, action kind and expected outcome; a control target is optional. | `out/main/index.js:733–787`; current guide REA evidence above and manual static reading. | SAW (static) |
| Guide checklist | Completed, current and upcoming steps have distinct presentation. Completed instructions can be expanded again. A user-confirmed current step offers Next or Done. | `out/renderer/assets/panel-DHxMl8DT.js:1820–1882,2000–2014`; checklist contract at `out/main/index.js:873–901`. | SAW (static) |
| Guide progress and cancellation | The coordinator preserves completed instructions, publishes the current plan and accepts pause, resume and end commands. Pause hides its mark; resume reassesses the current situation. | `out/main/permissions-C2fZeNd1.js:527–613`; `out/main/index-DpEY5uyy.js:8507–8530`. | SAW (static) |
| Guide checkpoints | Control observations are compared with expected outcomes. Unexpected changes, a missing target and an ineffective click can return the guide for replanning. User-confirmed expectations do not automatically pass. | `out/main/permissions-C2fZeNd1.js:807–870,959–972`; `out/main/index-DpEY5uyy.js:7996–8080`. | SAW (static) |
| Guide away lifecycle | Marks hide when the target is not foreground; prolonged absence changes the displayed state and eventually expires the guide. The inspected thresholds are two minutes for the away chip and ten minutes for expiry. | `out/main/index-DpEY5uyy.js:8398–8427`; `out/main/index.js:798–804`. | SAW (static) |
| Startup | The installed app exposes a separate onboarding document and conversation-panel document. Its panel reports that sign-in is unavailable. | Windows accessibility observation of the onboarding and panel windows on 14 November, using the installed ASAR whose hash matches above. | SAW (runtime) |
| Hosted history and authenticated guidance | The shipped client implies remote history and model services. Their retention, responses and actual authenticated behavior were not established. | Client contracts plus the unavailable sign-in surface; no authenticated feature flow was exercised. | INFERRED / unverified |

No Korso source, prompts, assets, branding or UI wording were copied into James.
The retained artifact and extracted code remain outside the source tree.

## Changes to James

### Conversation frontend and local state

`conversations.rs` classifies saved timestamps using explicit elapsed windows:
under 24 hours, from 24 hours to under seven days, seven days or older, and
unknown. Zero and future timestamps are unknown, so a clock adjustment cannot
make an invalid time appear freshly saved. These are original elapsed-time
choices, not a claim about Korso's calendar grouping.

`chat.rs` displays those groups and elapsed save times. Each group has stable
newest-first ordering, with the conversation ID breaking ties. Saved-chat search
remains literal, local and independent of inference. A separate current-chat
card is navigation and remains reachable when the saved search has no matches.
Returning changes only the page, history visibility and composer focus; drafts,
screenshots, attachments, pending proposals and archives are not consumed.
Deletion still requires confirmation and now identifies the selected saved chat.

No session-format or retention-policy change is introduced. Saved times describe
the existing archive timestamp, not a newly measured last-edit timestamp.

### Guidance frontend and state synchronization

The original James `GuidePlan` state replaces the earlier marks-only state.
It keeps up to three ordered instructions, the answer and screenshot identity,
optional numbered coordinates, user-confirmed progress and an independent
visibility mask. The conversation preview and Screen context share its checklist
and controls. A valid numbered answer can supply a checklist with `Marks: none`
or with marks disabled. The ordinary answer remains visible when no usable
numbered plan is available.

The checklist distinguishes completed, current and upcoming instructions.
Completing the current step advances locally; confirming the last step finishes
the guide. Pause blocks advancement and suppresses its numbers. Resume continues
the same plan, with screenshot identity checked before synchronizing marks.
End stops the plan without claiming that the remaining steps were completed.
These controls neither contact a model nor propose or execute an input action.
Completion records the user's confirmation; James does not verify that the
target app reached the expected outcome.

Hiding a number does not remove its instruction or complete a step. Progress
and visibility are separate state. The running step can carry a bounded
instruction label in the desktop overlay; other numbers retain their existing
presentation. This uses James's original overlay label support.

An already-open overlay receives the visible mark set after screenshot identity
is checked. Hiding all numbers closes a numbers-only overlay; user drawings are
preserved if present. Restoring numbers does not open an overlay automatically.
Invalid updates close stale overlay content. A replacement guide answer,
capture, discarded screenshot, conversation change or deletion of local data
clears the prior plan and its numbers. Progress and visibility remain in memory;
neither is added to the persisted session or restored as a live archived guide.

This deliberately uses controls within James rather than turning the native
click-through guidance overlay into an interactive copy of Korso's UI.

### Guidance protocol parsing

The old digit-extraction parser could turn `1 (-2, 3)` into step 1 at positive
pixel `(2, 3)`. The new parser requires the requested unsigned integer tuple
syntax. It rejects signs, decimals, exponents, overflow, extra fields, missing
delimiters and trailing prose. Existing bounds and duplicate-step checks remain.
Markdown emphasis around a properly formed metadata line is still accepted.
This is an original parser fix informed by the reference's validated-message
boundary, without copying its schema implementation.

The checklist reads numbered instructions from James's own model response; it
does not adopt Korso's tool schema or implement its automatic native-control
checkpoints, background window polling, away timer or replanning loop.

### Multi-file attachments

Korso's inspected pending-file contracts accept multiple files with per-file
removal. James now does too, as original work: an `attachments::Selection`
holds up to eight frozen files with stable IDs, per-kind aggregate byte
budgets (96 KiB of text, 16 MiB of images) and per-file removal. The whole
selection travels through one worker result, so a late result can never
replace files the user already selected, and the exact frozen selection is
restored when a chat request fails or is interrupted. The request sends the
screenshot first when one is attached, then the selected images in order,
through the shared multi-image serializer; crop and annotations apply only to
the screenshot. Attaching images and attaching the screenshot remain mutually
exclusive, as before for one image.

### Remaining frontend/backend gaps

Automatic guide checkpoint detection, guide questions/replanning,
authenticated services and the other gaps in the reconstruction report remain.
These changes do not establish a complete replica or comparative product
superiority.

## Verification and limits

Regression coverage exercises timestamp boundaries and unknown times; current
navigation with an unsent draft, screenshot and attachment; an empty saved-search
result; shared mark controls across chat and Screen context; independent hiding,
restore and replacement; discarding a capture; preserving user drawings; refusing
invalid overlay updates; and malformed model tuples. Checklist tests cover plans
without coordinates, current/upcoming/completed rows, confirmation of the last
step, pause/resume/end, visibility independent of progress, bounded current-step
labels and replacement of the active plan. GUI fixtures inspect that local
progress controls do not start a model worker or create an action proposal.
Tests drive the actual egui frontend and local model fixtures rather than the
separate HTML experiment. The existence of tests does not establish their final
pass status; use the validation record for the commands and results.

Final command results are recorded in [VALIDATION.md](VALIDATION.md).

## Completion pass, 14 November 2026

The multi-file attachment work above was finished in a later session on the
same day. It completed the `attachments::Selection` migration that the tree
was left mid-way through: the model request now sends every selected image
through one multi-image serializer call; the composer, attachment window and
conversation-context controls list each selected file with its own removal;
`ask_again`, new-conversation and data-deletion paths reset the whole
selection; and a failed or interrupted chat reply restores the exact frozen
selection that was consumed when the request was sent, closing the
`unsent_selection` lifecycle the earlier pass had opened. The `james-models`
multi-image tests, the new `attachments` selection tests and the affected GUI
fixtures were corrected to compile and pass (including a fixture whose helper
embedded a unique prefix in the asserted file names, and one that expected a
renamed file to change a frozen selection).

The guide checklist rows no longer use a check mark and a small triangle:
neither has a glyph in the fonts James ships, and the glyph-coverage test now
scans `guide.rs`, so the earlier symbols would have rendered as empty boxes.
Completed steps are struck through instead, and the current step uses the
covered large triangle. The strict workspace Clippy run also drove out an
unused accessor and a collapsible conditional from the same migration.

The finished tree passed the full workspace test suite (**361 tests**, 211 of
them desktop, four opt-in ignored), strict Clippy with warnings denied and
rustfmt on Windows ARM64 building the x86_64-pc-windows-msvc target under
Visual Studio Build Tools. These are James test-fixture results; they do not
establish physical-device behavior or Korso parity.

Not tested in this pass: authenticated Korso history/guidance, its hosted
backend, physical overlay transparency/click-through, mixed-DPI monitors, and
macOS/Linux native windows. The GUI fixtures establish James state and rendering
behavior, not physical desktop effects or comparative usability.

The standalone HTML experiment is outside the repository and is not shipped as
James. Codex authored the initial Rust changes. Subsequent implementation used
the Abliteration Flash model and then the user-selected Large V2 model to make
actual source changes through a scoped MCP bridge providing real REA operations
and allowlisted public James source read/search/edit access. These model-assisted
changes are original James implementation work, not recovered Korso source.

The provider received allowlisted public James source, structural REA results
and sanitized behavior notes. It did not receive private user files or raw Korso
source. The API credential, private local usage ledger and extracted reference
remain outside the repository. The user raised the cumulative task spend cap to
**$10**. This report does not freeze an interim spending figure: the private local
ledger and final task handoff record the accounted total. Organization-wide
balance changes can lag or include unrelated usage.
