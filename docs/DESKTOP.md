# James desktop v0.3 preview

James has a native Rust interface for local-model chat, optional screen context, individually reviewed Windows input, Windows speech, and live read-only R.A.I.N. Lab context. The Windows build remains an unsigned preview. The implementation is present; real-model quality and broad hands-on platform validation are separate acceptance gates.

## Install or run

The [desktop workflow](https://github.com/topherchris420/james/actions/workflows/desktop.yml) is configured to produce a portable `James-Windows-x64` artifact and a `James-Windows-x64-installer` artifact containing setup and a checksum. Download from a successful run containing these changes; a v0.3 public release is not implied. GitHub may require sign-in.

Run the setup executable and choose **Install**, then open James from the Start menu. Alternatively, extract the portable executable and open `james.exe`. Neither distribution includes LM Studio, model weights or the Lab. Setup is per-user; signing and clean-machine install/upgrade/uninstall validation remain outstanding. See [INSTALLER.md](INSTALLER.md).

For source builds, install Rust stable and native build tools (MSVC plus Visual Studio C++ Build Tools on Windows):

```sh
cargo test --workspace --all-targets --locked
cargo run -p james-desktop --locked
```

The native screen, input and speech operations require Windows. On other platforms they return an unsupported-capability error. A successful cross-platform compile does not imply those features are portable.

## Connect a model

1. Start [LM Studio](https://lmstudio.ai/), load a model and enable its local server.
2. Open **Settings** in James and enter the numeric port (usually **1234**).
3. Choose **Discover models**, select an actual loaded identifier, and save settings.
4. Write a question and choose **Send**, or press Ctrl+Enter in the draft.

James uses `/v1/models` and `/v1/chat/completions`. Replies are non-streaming. Model work runs off the UI thread with bounded requests. Stopping the UI operation does not guarantee that an already dispatched model request stops computing on the server. Protocol behavior is covered by mock HTTP tests; real LM Studio inference and vision quality have not been established by those tests.

Both model and Lab connections are numeric loopback only, with redirects and environment proxies disabled. There is no automatic cloud fallback, but the selected server may itself route to a remote provider or retain requests.

## Screen questions and computer input

In **Screen**, choose **Capture primary screen**. James minimizes and waits three seconds so you can show the target application. Inspect the image, then explicitly attach it to your next question. This requires a vision-capable model. Capture is one primary-monitor snapshot, not a screen feed; images are not saved by James.

You may click a point in the preview and review that click, prepare one supported key or printable-text insertion in **Actions**, or ask the model to **Suggest one action** using a fresh capture. Suggestions are parsed as a single supported action; arbitrary shell commands and action sequences are not accepted.

Review the exact input and choose **Allow once** or **Deny**. Allowing starts a five-second countdown; focus the intended target application during that time. The approval expires if dispatch is more than two seconds overdue, the wall clock moves backward, or the capture exceeds 60 seconds. Sleep or a paused UI cannot turn a delayed countdown into a later action: capture and review again. The worker preserves and checks this original deadline before recording intent and sending input. **Stop** or Escape in James cancels. There is no global stop shortcut. Review is specific to one input and does not authorize later actions.

The approval path uses `james-core`, a one-action budget and a flushed local intent record before Windows input. After an action, check what happened in the target application and choose **I checked the target — clear recovery block**. The marker also survives interruption or restart. An unknown result stays blocked until acknowledged; James never automatically replays it. Acknowledgment is your inspection, not automatic proof that the intended result occurred.

Input acts on the foreground application. There is no guaranteed process targeting, OS sandbox, multi-monitor control or autonomous action loop. A reviewed click or key may still submit or delete data. Keep the target consistent with the preview and inspect each result.

## Voice

Choose **Dictate** for one utterance lasting up to 20 seconds. Recognized text enters the editable draft; inspect it before sending. **Read last answer** uses Windows speech synthesis, and **Read new answers aloud** optionally speaks future replies. **Stop** cancels speech work.

Speech uses installed Windows speech components and an available recognizer/language; microphone permissions and device configuration must permit capture. Missing components or recognition failures surface as errors. No wake-word listener, continuous voice conversation or automatic speech-to-action execution is implemented. James does not save audio. Hands-on microphone and recognition accuracy testing remains pending.

## Live R.A.I.N. Lab context

Start the separate Lab development server. In **Lab**, enter its port (normally **5173**) and choose **Connect to Lab**. James polls `/api/rain/discovery` every five seconds, displays the current research state, and can include a bounded snapshot in a model request when you enable the attachment. Snapshots older than 15 seconds or unavailable snapshots block an attached request rather than silently reusing stale context. Disconnect clears the local snapshot and attachment selection.

This is the upstream `rain-discovery-view/v1` endpoint, verified against a real local Lab server. It is read-only: no experiment is authorized, started, stopped or modified by James. It does not synchronize research memory or turn simulator findings into real-world evidence. See [LAB.md](LAB.md) for source provenance, limits and reproduction.

## Storage and privacy

On Windows, conversation text and settings are saved as plaintext at `%LOCALAPPDATA%/Vers3Dynamics/James/session.json`; the non-Windows fallback is `~/.james/session.json` when `HOME` exists. The adjacent `action-intent.json` stores the latest reviewed action, including any text to insert, until you acknowledge it. Clearing conversation history overwrites the session history; it does not securely erase previous bytes or independently resolve an action marker.

Screenshots and microphone audio are not persisted by James. Explicit screenshot and Lab attachments are sent to the selected model service, which may log them. Model answers and your sent text become conversation history. No encrypted-at-rest or secure-erase guarantee is made.

## Validation still needed

The Lab adapter was checked against the actual upstream idle server; fixture tests cover its protocol and failure cases. Model HTTP tests cover protocol shapes and bounds, not a real inference session. Before treating this as a validated public release, exercise real text/vision models, speech devices and recognition accuracy, foreground input across target apps, accessibility, and clean-machine install/upgrade/uninstall. Signing and stronger outcome verification remain future work. Consult [VALIDATION.md](VALIDATION.md) for the recorded checks.
