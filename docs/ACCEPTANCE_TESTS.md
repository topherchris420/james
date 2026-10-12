# Acceptance tests

These are the end-user scenarios a release must pass. Each one lists what
to do, what must happen, which automated tests cover it, and the latest
recorded result. Automated tests are listed so you know what they already
check; **they never replace running the scenario on the named system with a
real model.** Record every run, including failures, in
[VALIDATION.md](VALIDATION.md) and update the status here.

## Rules for recording a result

- Use the actual released package (or a build of the exact commit) on the
  named OS and hardware, with a disposable user account or profile.
- Note: date, commit or version, OS and version, CPU, RAM, display (scale,
  monitors), model app and version, exact model ID, and timings.
- **Pass** means it ran there and behaved as expected. **Fail** means it ran
  and did not. **Blocked** means it could not be run (say why). **Not run**
  means nobody has tried. A result on one OS never counts for another.
- Screenshots only of disposable windows; never of credentials or private data.

Status key used below: ✅ Pass · ⚠️ Partial (passed in part, or only in a
container) · ❌ Fail · ⛔ Blocked · ⬜ Not run.

## Status summary (James 0.4.0, 2026-10-12)

| # | Scenario | Linux | Windows | macOS |
| --- | --- | --- | --- | --- |
| 1 | First-time install | ⚠️ container | ⬜ | ⬜ |
| 2 | No model software installed | ⚠️ container | ⬜ | ⬜ |
| 3 | LM Studio installed but not running | ⬜ (no LM Studio) | ⬜ | ⬜ |
| 4 | Server running without a loaded model | ⚠️ scripted server | ⬜ | ⬜ |
| 5 | Compatible local model connects | ⚠️ container, Ollama + llama.cpp | ⬜ | ⬜ |
| 6 | Chat with real streaming | ⚠️ container | ⬜ | ⬜ |
| 7 | Vision model receives an approved screenshot | ⚠️ container | ⬜ | ⬜ |
| 8 | User denies a proposed action | n/a (no input) | ⚠️ GUI test on CI runner | n/a |
| 9 | Approval expires before dispatch | n/a | ⚠️ automated only | n/a |
| 10 | Crash during an unresolved action | ⚠️ planted record | ⬜ | ⬜ |
| 11 | Model disconnects mid-reply | ⚠️ automated only | ⚠️ automated only | ⚠️ automated only |
| 12 | User stops generation | ⚠️ container, Ollama | ⬜ | ⬜ |
| 13 | History survives a restart | ⚠️ container | ⬜ | ⬜ |
| 14 | User clears retained information | ⚠️ container + GUI tests | ⬜ | ⬜ |
| 15 | R.A.I.N. Lab unavailable | ⚠️ container (real Lab connected; offline automated) | ⬜ | ⬜ |
| 16 | Update keeps compatible settings | ⚠️ container (0.3.0 → 0.4.0 `.deb`) | ⬜ | ⬜ |
| 17 | Nontechnical user follows troubleshooting | ⬜ | ⬜ | ⬜ |
| 18 | Keyboard-only and assistive technology | ⚠️ keyboard only, X11 | ⬜ | ⬜ |

"Container" = Ubuntu 24.04 container with Xvfb and software OpenGL, no
window manager, no GPU, no audio, driven with `xdotool`; see
[VALIDATION.md](VALIDATION.md#end-user-release-session-2026-10-12).

## Scenarios

### 1. First-time user installs James

1. On a clean account, download the package for the OS from the Releases page and check its SHA-256 against `SHA256SUMS.txt`.
2. Install it the way the [getting started guide](GETTING_STARTED.md#1-install-james) says (including the unsigned-build warning on Windows and macOS).
3. Open James from the Start menu, Applications, or the application menu.

**Expected:** James opens at **Welcome to James**, step 1 of 5. No terminal was needed. The data folder does not exist until a setting is saved.
**Automated:** CI installs both Linux packages in Ubuntu 22.04 and runs `james --version`; GUI test `first_run_setup_saves_each_step_and_resumes_after_a_restart`.

### 2. No model software is installed

1. Make sure nothing listens on ports 1234, 11434 and 8080.
2. In setup, choose **Get started**, then **My model app is running**.

**Expected:** "No model server answered on this computer (ports 1234, 11434, 8080)…" with what to start; **Next** stays disabled; nothing is installed or downloaded.
**Automated:** `find_prefers_a_server_with_a_local_chat_model`, `find_servers_*` in `james-models`.

### 3. LM Studio is installed but not running

1. Install LM Studio, quit it completely, then press **Find my model server** (and, separately, send a message with LM Studio selected in Settings).

**Expected:** Find reports no server. Sending says "James can't reach LM Studio on port 1234. Open LM Studio, load a model, and start its server on the Developer tab…".
**Automated:** `failures_come_with_a_next_step_and_the_servers_own_words`, `a_model_server_that_is_not_running_gets_advice_and_a_retry`.

### 4. A model server is running without loaded models

1. Start LM Studio's server with no model loaded (or Ollama with no models pulled), then press **Find my model server**.

**Expected:** "LM Studio is running on port 1234, but it has no models yet. Download and load a model in LM Studio, then try again." A working model choice is never replaced by an empty server. With LM Studio's just-in-time loading, a listed but unloaded model shows "loads on first use".
**Automated:** `describe_found` tests; LM Studio `/api/v0/models` parsing tests.

### 5. A compatible local model connects

1. Load a model (suggested: Gemma 3 4B) and press **Find my model server**.
2. Check the model line under the model picker.

**Expected:** The model is selected; its server's facts are shown ("can read images, 4096-token context window, loaded (according to Ollama)"); a cloud model is labelled "(runs elsewhere)" and never auto-selected; **Send a test message** gets a real reply and "It works".
**Automated:** `setup_finds_a_model_says_hello_and_keeps_the_test_out_of_history`, `a_cloud_model_is_flagged_before_anything_is_sent`, capability parsing tests recorded from real servers.

### 6. Chat works with real streaming

1. Ask a question that needs a long, formatted answer.

**Expected:** Text appears as the model writes it; a "thinking" note appears first for reasoning models; Markdown (lists, bold, code) renders; Copy and Ask again work; the answer is saved.
**Automated:** `a_reply_streams_in_renders_markdown_and_is_saved`, `ask_again_replaces_the_last_answer`, stream framing tests.

### 7. A vision-capable model receives an approved screenshot

1. With a vision model, open a harmless window, press **Guide me**; separately, capture on **Screen context**, look at the preview, tick **Attach**, and ask about it.

**Expected:** James hides for three seconds, captures once, and the answer refers to what is on screen. With a text-only model, Guide me is disabled and explains why. On macOS, record the Screen Recording prompt.
**Automated:** `a_screen_question_is_sent_without_the_conversation_history`, `a_text_only_model_cannot_be_asked_about_the_screen`, scaling and click-mapping tests.

### 8. A user denies a proposed action (Windows)

1. Capture a disposable app, prepare one key (or **Suggest one action**), read the review, choose **Deny**.

**Expected:** "Action denied. Nothing was sent." No action record exists; nothing happened in the app.
**Automated:** `review_screen::deny_sends_nothing_and_clears_the_review` (Windows CI).

### 9. An approval expires before dispatch (Windows)

1. Prepare an action, wait more than 60 seconds after the capture, then try **Allow once**; separately, put the computer to sleep during the countdown.

**Expected:** Allow once is disabled with "This review has expired…", or the countdown ends with "Approval expired before input. Nothing was sent." Nothing is typed.
**Automated:** `timing.rs` tests, `expired_ui_approval_never_records_intent_or_executes`, `expiry_during_dispatch_does_not_refresh_the_ui_approval`, `cancelling_the_countdown_sends_nothing`.

### 10. James crashes during an unresolved action (Windows)

1. Approve a harmless key in Notepad; during the countdown or right after input, end James from Task Manager. Reopen James.

**Expected:** "An approved action hasn't been checked yet." Computer control shows exactly what it was, in which window, when, and that the outcome is unknown. No new action is possible until **I checked the app — continue**. Nothing is replayed.
**Automated:** `an_unchecked_action_is_explained_and_cleared_only_by_the_person`, `a_record_from_a_crash_is_explained_and_never_ignored`, `durable_intent_precedes_effect_and_blocks_replay`.

### 11. A model disconnects halfway through generation

1. While a long answer streams, quit the model app (or stop its server).

**Expected:** The part that arrived is kept and labelled "[Reply interrupted: …]"; the status explains; **Try again** works once the server is back.
**Automated:** `a_disconnect_mid_stream_is_an_error_not_a_finished_reply`, loopback truncation tests.

### 12. A user stops generation

1. During a long answer press **Stop (Esc)**.

**Expected:** Stops within a second; the partial answer is kept with "[Stopped by you before the reply finished.]". Record whether the model app stops generating (Ollama 0.40.3 did).
**Automated:** `stop_closes_the_reply_and_keeps_what_arrived_marked_incomplete`, `abort_ends_a_stalled_stream_promptly_and_keeps_what_arrived`.

### 13. Conversation history survives a normal restart

1. Hold a conversation, quit James, reopen it.

**Expected:** Same model, settings and conversation (at most the last 80 messages). On macOS/Linux the folder is 0700 and files 0600.
**Automated:** `sessions_round_trip_and_history_can_be_left_unsaved`, `files_are_private_to_the_owner`.

### 14. A user clears retained information

1. Settings → Privacy: turn off **Keep my conversations on this computer**; then **Clear conversation…**; then **Delete all James data…**.

**Expected:** Each asks first where it destroys data. After deleting, setup starts again and only an unchecked action record (if any) remains. Check the folder by hand. Uninstalling on Windows asks whether to delete data (default No).
**Automated:** `turning_off_history_removes_the_saved_conversation`, `deleting_everything_asks_first_and_keeps_an_unchecked_action_record`, `delete_all_keeps_an_unresolved_action_record`.

### 15. R.A.I.N. Lab is unavailable

1. With the Lab stopped, press **Find the Lab**; then start it, connect, stop it again.

**Expected:** "R.A.I.N. Lab didn't answer…" with how to start it; once connected and then stopped, "Offline… Trying again in 5 s", backing off to 60 s; sharing refuses a stale snapshot. Chat works throughout.
**Automated:** `an_offline_lab_is_retried_gently`, `the_lab_view_explains_results_and_shows_what_would_be_shared`, `find_lab_checks_ports_in_order_and_skips_other_servers`.

### 16. An installed update preserves compatible settings

1. Install the previous release, set a model and hold a conversation; install the new release over it.

**Expected:** Setup is skipped, the model, settings and conversation remain; an old action record is still recognised. Record any permission prompts re-requested on macOS (the ad-hoc signature changes every build).
**Automated:** `old_sessions_load_with_safe_defaults`, v1 record test.

### 17. A nontechnical user follows the troubleshooting guidance

1. Give a person who has never used a local model the release and the [getting started guide](GETTING_STARTED.md), no other help. Break one thing (e.g. quit the model app) and ask them to recover using the guide.

**Expected:** They reach a first answer and recover. Record time taken, every hesitation, and the words they used.
**Automated:** none possible.

### 18. Keyboard-only and assistive-technology navigation

1. Unplug the mouse. Complete setup, ask a question, open each page, change a setting, and (Windows) deny an action, using only the keyboard (F1 lists shortcuts).
2. Repeat with NVDA or Narrator (Windows), VoiceOver (macOS) and Orca (Linux). Record what is announced for buttons, fields, the model line and replies.
3. Try high contrast, 200% text and reduced motion.

**Expected:** Every control is reachable with Tab and has a visible focus ring; the screen reader names controls and reads messages; nothing is cut off at 200%.
**Automated:** `the_keyboard_alone_reaches_every_main_control`, `shortcuts_switch_pages_and_show_their_own_list`, `the_layout_holds_at_small_sizes_high_dpi_large_text_and_every_theme`, the theme contrast test.

## Results log

Add one row per run. Earlier sessions are in [VALIDATION.md](VALIDATION.md).

| Date | Version / commit | System | Model app / model | Scenarios | Result | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| 2026-10-12 | 0.4.0 (pre-tag) | Ubuntu 24.04 container, Xvfb, 4 vCPU, no GPU | Ollama 0.40.3 source build / `gemma3:4b`; llama.cpp b11564 / SmolVLM2-500M, Qwen2.5-1.5B | 1, 2, 4–7, 10, 12–16, 18 (keyboard) | Partial (container) | See VALIDATION.md. Found and fixed: composer refused typing; Guide me sent history; invisible emphasis; two missing glyphs |
