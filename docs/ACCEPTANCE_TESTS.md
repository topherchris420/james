# James acceptance test protocol

**Do not mark a scenario passed unless it ran on the actual named OS, hardware, build and model.** Use disposable apps/data. Record date, Git commit SHA, platform and version, CPU architecture, RAM, Rust version (if compiled), server product/version, exact model ID, screenshots only with permission, measured timing and any observed failures.

## Baseline

1. Fresh install or launch on an isolated test account. Capture launch time, warnings, accessibility issues and missing libraries.
2. Start LM Studio, Ollama or llama.cpp locally with an already-loaded model. Keep the API bound to loopback. Do not change firewall rules or expose its port.
3. Probe the model list and optionally real inference on the **same machine running that server**:
   ```powershell
   pwsh -File scripts/test-local-provider.ps1 -Provider LMStudio
   pwsh -File scripts/test-local-provider.ps1 -Provider LMStudio -Model '<exact loaded ID>'
   ```
   For Ollama, use `-Provider Ollama`. For llama.cpp, use `-Provider LlamaCpp` and the configured port if not 8080.
4. Open James **Settings**, select the matching provider, use **Discover models**, pick the actual ID, ask a non-sensitive question. Record whether reply is correct and whether the UI freezes. Confirm an absent, stopped or incompatible server surfaces a bounded error.
5. Restart James. Confirm the selected provider, model, port and bounded conversation survive. Load an old v0.3 `session.json` in a disposable profile; verify it defaults to LM Studio without losing messages.

## Vision, permissions, control

Only on the currently implemented Windows platform:

1. Capture a non-sensitive disposable app, inspect the screenshot and explicitly attach to a compatible vision-capable model. Record response and model vision capability separately.
2. Prepare one harmless action. Inspect the exact click, key or printable text, deny once, and verify no effect.
3. Prepare again; allow once with focus countdown, verify the **foreground** target yourself. There is no guaranteed target lock.
4. Verify no second action without a fresh approval; test screen staleness, cancellation before dispatch, sleep/resume and foreground focus changes.
5. Restart with an unresolved action marker. Verify James blocks another action until manual inspection/acknowledgement.
6. Test speech/dictation with actual microphone and permissions. Inspect draft before sending.

## Voice and guidance (every platform)

1. **Talk:** hold a three-turn spoken conversation. Record recognition errors, answer latency and whether spoken answers sound natural. Confirm "stop", Esc, End conversation and 15 s of silence each end the loop, and that "How do I stop the timer?" is treated as a question.
2. Say "allow once" or "click OK" during Talk while an action proposal is pending. Confirm nothing is approved or executed.
3. **Guide me:** in a disposable app, confirm the answer names visible labels and gives steps without producing an action proposal.
4. **macOS/Linux listening:** with whisper.cpp absent, confirm Settings explains what to install and Talk/Dictate are disabled. Install it and a model, choose **Check listening setup**, and confirm the reported model and recorder. Record mic permission prompts, keyboard clicks or desk taps falsely starting an utterance, words clipped at the start or end, and whether any audio files remain in the temporary folder afterwards.
5. **macOS/Linux capture:** record the Screen Recording prompt (macOS) or which Linux tool captured, and whether multi-monitor setups capture the expected display.

On macOS/Linux confirm the UI does **not** imply computer input support, and that text chat plus optional read-only Lab context remain accessible.

## Reliability and safety

Simulate a lost local model connection during request, unavailable Lab, malformed response, missing or corrupted session, OS sleep/resume, interrupted installer, permission revocation and a stale approval. Confirm no automatic retry of a possibly executed computer action. Verify screenshot pixels and raw microphone audio are not persisted by James. Confirm any provider logs and storage files match the privacy description.

Use native assistive technology for keyboard navigation, screen readers, contrast, font scaling, focus, Unicode text, recovery, reduced motion and permission language. Run fresh install, upgrade and uninstall, inspecting history retention. Confirm model/voice/vision feature claims are specific to tested provider and hardware.

## Fill out one row per device and experiment

| Test date | Commit | OS / version / CPU / display | Provider / model | Scenario | Pass, fail, blocked or not run | Evidence / notes |
| --- | --- | --- | --- | --- | --- | --- |
| *(enter actual date)* | *(SHA)* | *(exact system)* | *(exact model)* | *(scenario)* | Not run | No synthetic result |
