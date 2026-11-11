# Korso 0.7.8 reconstruction and James replication

Assessed 10 November 2026. This pass goes beyond the earlier package inventory:
REA extracted the selected Korso artifact, successfully parsed two selected
JavaScript source sets, and retained their static analysis. Extracted code was
also read as inert text. The feature reconstructions below are **manually
inspected static artifact facts**, with inferences and unknowns identified
separately. Korso was not
installed, launched, signed in to, or exercised. No backend was contacted and no
Korso source, prompts, artwork, or assets were copied into James.

James implements original Rust behavior inspired by the recovered interaction
requirements. This is a bounded replication pass, not a claim of complete
functional, visual, or behavioral equivalence.

## Artifact and evidence identity

| Item | Identity |
| --- | --- |
| Package | `@korso/desktop`, product Korso, version **0.7.8**, entry `out/main/index.js` (`package.json:1–18`) |
| Selected ASAR | `C:\Users\chris\.cache\james-korso\2026-10-11\application\resources\app.asar` |
| SHA-256, independently read in this pass | `20f351cbd359b6e3f85838e7e095e95765aa3ceca7133cdcf90b46fdd347aa22` |
| Retained inventory parent | `ev_c8a680c9e0a29c0f68c2d0c9368a6b5f31bf90f3652940255a27362990bea232` |
| REA extraction | `ev_5f917b280ea4095a3d8571f4f305934cfad4a9e72f193e1dab533ec8a2e58f6c`; 2,044 extracted files |
| Initial ARM JavaScript attempt | `ev_7814d1e393623a26c3dfc164c326dd5947ddff1cb4fb019ac907640f14392394` |
| Successful x64 capture/overlay analysis parent | `ev_797f503a0e01a6c83194f4e96951d01d4db001bf082a21559b8456ea08e45764` |
| Successful x64 core/panel analysis parent | `ev_0f530b51b4b7a2a46140e7ec7c7997b3d35f5b1ea299eb6bdd52341f4f30e886` |
| Inspection root | `C:\Users\chris\AppData\Local\Temp\rea-extracted-0b9a787a-f0a1-40ca-a804-67057248aa24` |

The initial attempt used an unsupported ARM parser runtime and had zero AST
coverage; its full CLI error output was not retained. That blocker was recovered
by invoking REA with the existing
`C:\Users\chris\tools\node-x64\node.exe` runtime. No new analysis engine was
installed. **The earlier zero AST result did not establish missing features.**
The later passes parsed every selected file with zero parse failures.

| Successful pass | Parsed JavaScript files | Visited syntax AST nodes | Static findings | Parse failures | Whole-graph coverage |
| --- | ---: | ---: | ---: | ---: | --- |
| Capture/overlay/tab | 8 | 33,269 | 186 | 0 | Partial; 1,764 inventoried files outside this selection |
| Core/panel/onboarding | 10 | 784,582 | 2,063 | 0 | Partial; 1,762 inventoried files outside this selection; semantic retention truncated |

Syntax AST node counts measure parsed syntax, not retained semantic graph nodes,
executed code, or verified behavior. Both application and semantic graph coverage
remain partial: dependencies outside each selected set, dynamic expressions and
unresolved relationships remain unknown. The capture pass reports 1,760
application unknowns and 27,715 semantic unknowns, with no retention truncation.
The core pass reports 1,758 application unknowns and retains 40,000 semantic
unknowns with additional occurrences omitted. Its semantic-node and
semantic-unknown budgets are 40,000 each overall and 8,000 each per file; alias
selection expansion is bounded at 512 additional selector states. Its syntax
coverage is complete for the selected files even though semantic examination is
limited in five of the ten. Raising syntax counts does not remove these bounds.

The exact capture source selection was:

```text
out/main/capture-exclusion-BlboVEiP.js
out/main/displays-Bd5hinow.js
out/main/host-BxHpPclI.js
out/preload/capture.js
out/preload/overlay.js
out/renderer/assets/capture-CN6r0Fli.js
out/renderer/assets/overlay-BdilAO27.js
out/renderer/assets/tab-DPCu52d-.js
```

The exact core source selection was:

```text
out/main/index.js
out/main/primary-wShsXXbd.js
out/main/permissions-C2fZeNd1.js
out/main/index-DpEY5uyy.js
out/preload/panel.js
out/preload/tab.js
out/preload/onboarding.js
out/renderer/assets/panel-DHxMl8DT.js
out/renderer/assets/onboarding-BT7lNSWM.js
out/renderer/assets/contracts-381X8vKS.js
```

The exported evidence files are retained under
`C:\Users\chris\.cache\james-korso`. Byte sizes and SHA-256 values were read
from these exports after the successful runs:

| Evidence export | Bytes | SHA-256 |
| --- | ---: | --- |
| `2026-11-10-x64-capture-analysis.json` | 78,664,911 | `8fd49ff018ef172f5d18d2b2cc9f78d92ddc3e55a0c1a2d5982337370b8c0775` |
| `2026-11-10-x64-core-analysis.json` | 111,060,892 | `c90c08e7bc64ce2725ef97dcabf80d9b9a07242d2ce0d3388af70bdebe05fb05` |

Their UTF-16LE CLI logs are `2026-11-10-x64-analysis.log` and
`2026-11-10-x64-core-analysis.log`. The final JSON records contain summary views
and export locations. A summary view's complete-within-view status concerns that
projection, not complete application or semantic coverage. The full exports,
rather than a possibly truncated tool preview, retain the parent analysis.

Both successful passes retain the same six contradictions between declared
ASAR integrity metadata and observed native-file bytes:

```text
node_modules/@img/sharp-win32-x64/lib/libvips-42.dll
node_modules/@img/sharp-win32-x64/lib/libvips-cpp-8.18.7.dll
node_modules/@img/sharp-win32-x64/lib/sharp-win32-x64-0.35.5.node
node_modules/@korso/capture-host-native/build/Release/capture_host.node
node_modules/uiohook-napi/build/Release/uiohook_napi.node
node_modules/uiohook-napi/prebuilds/win32-x64/uiohook-napi.node
```

Those bytes are classified observed-untrusted. Parsing selected JavaScript does
not resolve the contradictions, establish their cause, or prove maliciousness.
The feature-level source references below remain manual readings; the aggregate
REA results do not automatically certify each manual conclusion or establish
runtime reachability, policy enforcement, native effects, or backend behavior.

All Korso paths and line numbers below are relative to this exact extraction.
They identify bundled files, not an upstream source checkout. Existing
[research qualifications](OBSERVATIONS_AND_ASSUMPTIONS.md) and the
[historical experimental assessment](BEHAVIOR_EVIDENCE.md) still apply. This pass
does not resolve the earlier native-file integrity contradictions or establish
native implementation behavior.

## Process and surface architecture

The package metadata and actual window/bridge construction establish an Electron
desktop with distinct surfaces, rather than one ordinary chat page.

| Surface | Static responsibility and source |
| --- | --- |
| Main entry/contracts | `out/main/index.js` defines typed message, tool, screen, history and settings contracts; `package.json` selects it as the entry. Its tool-input registry is at `index.js:2585–2645`. |
| Main coordinator | `out/main/primary-wShsXXbd.js` wires the session, history, voice, dictation, native platform, account gates, windows and keyboard callbacks. Examples: history wiring at `:39500–39536`, dictation at `:40198–40251`, keyboard callbacks at `:40308–40404`. |
| Conversation panel | `out/renderer/assets/panel-DHxMl8DT.js` renders conversation/history/settings. Its preload exposes a schema-checked `korso` bridge (`out/preload/panel.js:11465–11529`). The renderer issues commands and consumes state; it does not receive arbitrary Node access through this bridge. |
| Edge tab | A separate BrowserWindow is created at `primary-wShsXXbd.js:37778`. Its renderer is `out/renderer/assets/tab-DPCu52d-.js`; a dedicated preload exposes its bridge at `out/preload/tab.js:11417`. It opens/collapses the panel, presents work/recording state, and hosts optional widgets. |
| Screen overlay | `createOverlayWindow` at `primary-wShsXXbd.js:36946` creates the drawing surface; `out/preload/overlay.js:166` exposes a dedicated validated overlay bridge. `out/renderer/assets/overlay-BdilAO27.js` draws the user trail and assistant marks. |
| Capture/audio host | `out/preload/capture.js:197` exposes `korsoCapture`. Screen acquisition helpers live in `out/main/host-BxHpPclI.js`; microphone processing is present in `out/renderer/assets/capture-CN6r0Fli.js:327–780`. |

The shared window preferences explicitly set a memory partition, sandbox,
context isolation and disabled Node integration
(`primary-wShsXXbd.js:37515–37529`). These are inspected configuration values,
not proof that the entire application is a security sandbox. Native helpers,
IPC sender checks, external services and all execution paths require separate
analysis.

The dependency list includes a native capture host, image processing, input
hooks, reporting and updating. Presence alone does not establish that every
component runs in every session.

## Screen context, gestures, and visible guidance

The display catalogue assigns the primary display `screen1`, with other displays
ordered by position/id. Pointer lookup uses a containing or nearest display;
physical rectangles choose the display with greatest overlap
(`out/main/displays-Bd5hinow.js:23–99`). This is more specific than the older
public descriptions, but the complete capture-trigger routing has not been
exercised to resolve every primary-versus-pointer case.

The host selects an exact display id, converts physical crop boundaries with
floor/ceil rounding, clamps to the display, and rejects an offscreen rectangle.
Native BGRA acquisition checks byte length, approximate geometry and samples
before a desktop-capture fallback
(`out/main/host-BxHpPclI.js:276–308,363–448,457–485`). These checks are code
observations; their effectiveness with real drivers and mixed DPI is unknown.

The overlay uses CSS device-independent geometry with a device-pixel-ratio
transform (`overlay-BdilAO27.js:1376–1386`). User points form a fading trail.
Assistant drawing supports rings, boxes, underlines, arrows, labels, paths and
polygons, with staged animations and placement logic that avoids existing notes
(`overlay-BdilAO27.js:1–110,1045–1061,1125–1273`). Its preload bounds draw batches
and shape data: eight shapes per draw, 60-character labels, 64-point paths and
eight-point polygons (`out/preload/overlay.js:3–10,92–188`).

Ordinary unpinned marks expose a hover dismiss control. Mark hit testing,
different arrow/large-mark geometry, pinned marks and tour exceptions are
implemented (`overlay-BdilAO27.js:715–848,1274–1323`). Main's keyboard callbacks
also distinguish outside clicks, scrolling and Escape when clearing marks
(`primary-wShsXXbd.js:40358–40384`). Persistent assistant drawing and a temporary
user gesture are separate mechanisms.

The inspected overlay code does **not** establish that a user trail is baked
into the image sent to a model. James's new point/freehand marks are deliberately
rendered into the attached PNG by original Rust code, after applying its chosen
crop. That is an independent implementation decision. It offers an inspectable
way to ask about selected content without claiming Korso uses the same wire
representation.

## Conversation lifecycle and retained history

Korso's packaged UI has a real saved-conversation list. `HistoryRow` renders a
title, time, current selection and delete control
(`panel-DHxMl8DT.js:4027–4094`). `History` requests the list, renders grouped rows,
shows loading/error/empty states, offers retry, confirms deletion, and pages
older results (`panel-DHxMl8DT.js:4112–4229`). Opening the current row returns to
chat; opening another sends its conversation id. This is executable bundled UI
logic, not a feature inferred solely from labels.

`createHistorySession` manages list/read/create/delete calls, aborts superseded
requests, checks account generation, bounds its local window, and applies a
20-second operation deadline (`primary-wShsXXbd.js:24131–24340`). Contracts
contain a title, update time, revision and active flag; pages contain attempts,
pagination and context status (`index.js:5180–5285`). History pages allow at most
50 items and the client keeps a 200-item window. The title's backend derivation
and total server retention are unknown.

Main routes open to leaving the current request and adopting the selected page;
deleting the currently open record clears the selection
(`primary-wShsXXbd.js:39506–39535`). Reset/new conversation stops live voice,
clears marks, invalidates the active request, clears drafts/transcript, advances
generation and discards the current conversation reference. It does not issue
the history-delete operation (`primary-wShsXXbd.js:28761–28818,29877`). Adopting
a page reconstructs displayed turns and clears the editable question.

No conversation-search field or command was found in the inspected History
component and history contracts. Search elsewhere in this panel concerns apps,
languages or widget cities. This is a bounded negative finding, not proof that
every product surface lacks search.

James previously stored one current message vector and used New conversation to
clear it. This pass adds a local archive with generated ids, first-user-message
titles, most-recent ordering, restore and delete. It preserves old session files.
Its policy is intentionally different: up to 20 previous conversations, with
the **current and archived messages sharing** the existing 80-message/512 KiB
text budget; the current conversation has priority. The serialized file remains
capped at 4 MiB. Disabling history strips current messages and all archives on
both load and save. Images, drafts and action authorizations are not archived.
See `crates/james-desktop/src/conversations.rs` and `session.rs`.

The artifact also holds unsent file attachments separately from conversation
state. Classification and request conversion accept text, supported images and
PDFs, enforce per-file/aggregate limits, and publish rejection explanations
(`primary-wShsXXbd.js:23838–23914`). Panel state receives name/type/size summaries;
main retains the attachment bytes (`index.js:4381–4387`). This pass does not
establish the backend's parsing or retention of those files. James file
attachments beyond its explicit screenshot are remaining work.

## Talk, spoken answers, live mode, and dictation

The artifact distinguishes three user flows:

1. **Ask by holding a talk chord.** Main forwards press/release into the session.
   A short keyboard hold under 500 ms is canceled rather than treated as a
   completed request (`primary-wShsXXbd.js:28085,30020–30054`). Keyboard entry
   and microphone/account admission are explicitly coordinated at
   `primary-wShsXXbd.js:40308–40351`.
2. **Hear answers and continue talking.** The `Voice` class coordinates press,
   release, exchange context, playback, interruption, rollover and a grace clock
   (`primary-wShsXXbd.js:25235–25636`). A separate live-mode coordinator gates
   hands-free mode, chooses auto-off timing, handles microphone/refusal faults
   and configures talking over playback (`:27503–27644`). Settings/contracts
   distinguish voice enabled, currently speaking, voice unavailable, live
   enabled and live resting/listening/speaking/fault states
   (`index.js:4422–4487`). Actual speech quality, network timing and service-side
   processing were not tested.
3. **Dictate into another app.** `createDictationController` snapshots destination
   context/focused editability, starts audio, emits recording/processing state,
   stops and transcribes on release, and either pastes recognized text or keeps
   it when the destination is not editable
   (`primary-wShsXXbd.js:31672–31956`). Requests have generation checks,
   cancellation, release budgets and audio cleanup. A dictation lock changes
   the recording mode; suspend/lock cancels it.

The clipboard-paste implementation snapshots prior clipboard formats, stages
text (and optionally rich text), serializes deliveries, sends paste, and restores
the previous clipboard with explicit restoration errors. Windows exclusion
formats are attempted; fallbacks exist
(`out/main/permissions-C2fZeNd1.js:6211–6427`). This is not proof that clipboard
history is always excluded or that every app receives the text correctly.
Dictation context can include an app executable name and window title
(`index.js:5495–5505`); backend use of that context is unknown.

Audio-host code converts input into mono 16 kHz PCM16 chunks and provides level
measurements. Device choice/warming logic distinguishes Bluetooth devices
(`capture-CN6r0Fli.js:327–780`). No microphone or playback experiment occurred.

James already provides local Talk, single-utterance Dictate into its own composer,
read-aloud and installed reading voices. It does not reproduce Korso's remote
voice exchange, grace-window streaming or dictation into another app. This pass
protects a typed draft and its screenshot attachment while a Talk utterance is
submitted; spoken input still cannot approve an action.

## Approval, tools, and undo boundaries

The recovered approval policy has **Always**, **Auto**, and **Never** modes
(`index.js:3089`; `primary-wShsXXbd.js:16240–16277`). These names do not mean
every tool is treated identically:

- Connection tools use a separate connection card. Silent/read tools can run
  without a card unless a local check fires, including in Always mode. Hover,
  text reads and widget-list reads have explicit unchecked paths
  (`index.js:2482–2508`; `primary-wShsXXbd.js:16486–16505`).
- Auto combines local checks and supplied ratings. Ratings distinguish allow,
  card, block and several unavailable-rating reasons. An unrated script can be
  blocked under the packaged shell-first setting. The rating service's actual
  decision procedure is not present here.
- Never still presents cards for permanent changes and locally identified
  catastrophic operations (`primary-wShsXXbd.js:16223–16254`). It is not a
  universal permission bypass. Repeated blocked decisions have a bounded
  fallback to a review card (`:16255–16268`).
- The settings reader migrates a previous Always state to Auto unless it has an
  explicit user-choice marker (`primary-wShsXXbd.js:91–110`). This is a
  version-specific static migration finding, not an observed upgrade.

Local checks cover executable/data/startup paths, permanence, opened destinations,
settings and app actions (`primary-wShsXXbd.js:16326–16417`). Clicks can request a
second rating when the initial result is a card or certain rating failures,
using the identified control/window (`:16507–16584`). These mechanisms do not
establish that every unsafe action is caught.

The approval gate binds a decision to the current request and exact tool-use id;
stale answers do not resolve another pending card. Awaiting review pauses the
run budget, and missing/failed review callbacks deny
(`primary-wShsXXbd.js:16425–16484`). Cards can include a target, control name,
diff/preview, reason, app identity, arguments and an editable email. Hiding the
panel cancels an outstanding approval (`:29963–29968`).

The typed tool registry is substantially broader than James's desktop input:
screen reads/actions, clicks, keyboard/text, screenshots, files/directories,
scripts, launching/opening, settings, installed apps, connected app actions,
widgets, guides and feedback (`index.js:2585–2645`). This establishes accepted
contract families, not that every tool is enabled on every platform/account or
that every registered operation was traced to its native effect. For example,
app-tool availability is suppressed when elevated in the inspected adapter
(`permissions-C2fZeNd1.js:3736`).

Text undo has more specific boundaries than a generic Undo button. The packaged
`TextLedger` tracks per-conversation edit ids, size limits, evictions, missing
entries, untracked edits and closed windows. Its `last` lookup avoids silently
undoing an older edit when the newest cannot be recovered; it is cleared on
account/application cleanup (`permissions-C2fZeNd1.js:1663–1845,1995`). The text
contracts and refusal handling distinguish changed field versions, moved focus,
passwords, ambiguous edits and uncertain/partial writes
(`index.js:1091–1124`; `permissions-C2fZeNd1.js:3371–3426,3694–3734`).
The whole text-edit/undo effect was not traced end to end in this pass.
General file-backup completeness, undo of scripts or external effects, and crash
recovery remain unknown. The earlier public backup claim must not be promoted
to a verified universal rollback guarantee.

James retains its existing one-action Windows review, expiry/target checks,
durable intent and human reconciliation. This replication does not add Auto or
Never, scripts, connected-account actions, or unattended execution. The second
James model pass remains advisory and cannot authorize input.

## Tab, shortcuts, and configuration

Korso's tab is a draggable edge surface. Its recording state uses a waveform and
lock; work states cover capture, transcription, compaction, streaming, approval
and tools (`tab-DPCu52d-.js:801–1068`). Idle widgets are suppressed while recording
or working. The bundle contains now-playing, progress/notice/pet presentation and
a live-mode drop/notch (`tab-DPCu52d-.js:1–278,356–800,1011–1071`). This pass did
not validate media controls, feeds, timers, reminders, weather or pets end to end.

Three configurable shortcut actions are defined: hold-to-talk, panel toggle and
dictation. Their schemas require distinct modifier pairs and define separate
Windows/macOS defaults (`index.js:3913–3955`). James's opt-in global summon is a
new implementation with its own binding and registration limits; it is not a
recreation of all Korso chord/lock gestures. James's compact conversation view
also remains a normal native app view, not the same edge-tab/widget architecture.

Additional recovered settings include chosen microphone, speech languages, card
pinning, screen drawing/color, language, panel size, startup, analytics, voice
speaker, live auto-off and talking over playback
(`index.js:4373–4500`; `primary-wShsXXbd.js:65–110`). Interface languages are
English, Japanese and Chinese; live auto-off choices are 5, 10, 30 minutes or
never, defaulting to 10 (`index.js:4332–4335`). Values and fallbacks are static
facts; actual OS persistence, startup behavior and reporting have not been
verified by execution.

## Replication checklist

“Baseline” means present before this pass. “New” means original James source
added in this pass; integration and verification are recorded separately.
“Planned” identifies remaining parity work, not a promise of a finished feature.

| Recovered requirement | James status in this pass | Remaining boundary |
| --- | --- | --- |
| Typed/streamed conversation, Stop, retry, Markdown | Baseline | No equivalence test against Korso |
| New conversation preserving earlier chats | New local archive | Different storage/retention policy |
| Conversation list, title, resume, delete | New | Local first-question title; no remote revision/multi-device semantics |
| Text/image/PDF attachments | New | One local file at a time; PDF text extraction only, no OCR; 16 MiB file and 24 KiB text caps |
| History search | Unknown in Korso inspected surface | Not claimed as recovered behavior |
| User capture and preview | Baseline | Physical multi-monitor/DPI validation remains open |
| Windows monitor choice | Baseline | Unix picker/capture equivalence incomplete |
| Region questions | Baseline, with new fail-closed image preparation and crop preview | A failed crop must not expand transmitted scope |
| Point/freehand/circle context | New original PNG annotations, undo/clear marks | Korso overlay-to-model representation unknown |
| Assistant numbered screenshot guidance | Baseline | Model coordinates may be wrong |
| Persistent live desktop shapes/dismissal | Planned | James marks remain on its captured-image view |
| Guided checklist tied to live controls | Planned | Baseline Guide me is a bounded textual/screenshot answer |
| Compact assistant surface | New compact conversation view | No draggable edge tab or widget engine |
| Global summon | New opt-in registration | Platform scope and binding differ from Korso |
| Configurable hold/tap/dictation locks | Planned | Not reproduced by summon |
| Local Talk, Dictate and read-aloud | Baseline | No recorded microphone parity experiment |
| Preserve draft during Talk | New | Does not add voice authorization |
| Dictation into another app/clipboard restoration | Planned | James Dictate targets its composer |
| Live voice exchange, grace window, talk-over | Planned | James uses its existing local voice loop |
| Microphone selection/speech-language picker | Planned | Existing platform facilities retain their limits |
| Exact single-input review and recovery | Baseline James design | Korso's broader tool effects were not reproduced |
| Advisory second action review | Baseline James design | Cannot approve an action |
| Always/Auto/Never tool modes | Planned parity investigation only | James keeps explicit review per input |
| Scripts/files/app launching/settings tools | Planned | Schema inventory is not a finished James tool system |
| Connected apps/email action editing | Planned | Service/backends and external effects unknown |
| Text edit ledger and guarded undo | Planned | Universal rollback is unestablished |
| File backups/script rollback/crash guarantees | Unknown | No verified general transaction/recovery contract recovered |
| Edge-tab media/weather/pets/timers/reminders | Planned | Bundle/UI presence; end-to-end behavior unverified |
| Account/billing/usage/backend ratings | Outside this local replication pass | Service implementation unknown |
| Installer/update/signing parity | Outside this pass | Existing James release readiness limits apply |

## What remains unresolved

This investigation answers the principal client-surface architecture and traces
specific history, approval and dictation state transitions from bundled code.
It does not establish actual screen/audio/network payloads, permissions on a
physical desktop, real input effects, latency, reliability, provider retention,
service-side prompts/models, or all native implementations. It does not compare
the products experimentally. A later runtime pass would need controlled screen,
microphone and disposable target-app fixtures, declared actions, and retained
results before promoting any of those behaviors to observed execution.

## James validation, 10 November 2026

The integrated workspace test run passed **239 tests**, including **103 desktop
tests**; four opt-in tests were ignored. Focused desktop regressions exercise
conversation archive/open/delete, disk privacy, preserving a typed draft and its
attachment during a spoken question, refusing an invalid crop before chat HTTP,
and inspecting the actual decoded annotated/cropped PNG sent to a scripted local
model server. Compact-view tests inspect navigation and viewport commands.

These are James test-fixture results. They do not establish physical microphone
behavior, global shortcut registration in daily desktop use, all supported
platforms, or parity with Korso. Static inspection of Korso does not validate
James, and passing James tests do not validate Korso. Build and lint results are
recorded below.

The Windows GNU desktop build and workspace Clippy with warnings denied passed.
Native Windows inspection using an offline fixture confirmed the full conversation
view, compact companion layout, and history dialog render correctly. Opening an
archived fixture restored its conversation, and Ctrl+Shift+Space switched the
focused full window into the compact companion. Cross-app focus behavior was
not exercised. This is not
a cross-platform or real-model validation. The fixture used `JAMES_DATA_DIR` to
isolate its `session.json` from the normal profile; this optional environment
variable selects the session directory for testing or a portable profile.

The final inventory/extraction evidence was exported to
`C:\Users\chris\.cache\james-korso\2026-11-10-final-inventory-extraction.json`
(6,832,896 bytes, four records), and the REA session was closed. The two selected
JavaScript analysis bundles above remain separate evidence artifacts.

Further implementation adds local file attachments: UTF-8 text, extractable PDF
text, PNG, JPEG and WebP. Files can be dropped onto James or selected by path;
they are sent only with the next chat request and are not retained in history.
Automated tests exercise real PDF extraction, request payloads, attachment
privacy and preserving attachments during voice questions. Image-only PDFs need
external OCR. This does not establish Korso attachment equivalence.
