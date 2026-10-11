# James platform support and distribution

Status: **developer previews, not a universal production release**. A successful compilation, CI run, or archived binary is not a clean-machine installation, accessibility or native-permissions validation.

| Platform / architecture | Build pipeline | Native screen / input / speech | End-user status |
| --- | --- | --- | --- |
| Windows 10/11 x64 | Existing GitHub Actions executable and per-user installer, with tests | Windows-only fixed helpers; approved one action at a time | Preview; installer lifecycle and real-model integration still require hands-on verification |
| Windows 11 ARM64 | No native ARM64 build established | x64 compatibility not validated on all devices | Unsupported as a native target; compatibility is unverified |
| macOS (runner architecture) | macOS CI build and portable archive now configured; check actual run results | Screen capture (`screencapture`), speech (`say`) and whisper.cpp listening implemented, not device-validated. No computer input | Experimental source/portable build pending test results; unsigned, unnotarized |
| macOS Apple Silicon and Intel, independently | CI runner only establishes its *actual* architecture | As above | Other architecture unverified |
| Linux x64 | Ubuntu CI build and tarball now configured; check actual run results | Screen capture (gnome-screenshot, spectacle, grim, ImageMagick), speech (espeak-ng, spd-say) and whisper.cpp listening implemented, not device-validated. No X11/Wayland input or portal capture adapter | Experimental; display server and system library dependencies vary |
| Linux ARM64 | No native ARM64 CI or hardware validation | Not implemented | Unsupported / not validated |
| Browser/PWA | Not implemented | Browser cannot receive unrestricted desktop control | Not available |
| Android/iOS | Not implemented | No device control | Not available |

## Local model protocol choices

In **Settings**, select **LM Studio**, **Ollama**, **llama.cpp server** or **Other OpenAI-compatible server**, then load/start a model in the chosen runtime and discover its ID.

- LM Studio: `http://127.0.0.1:1234`, OpenAI-compatible `GET /v1/models`, `POST /v1/chat/completions`.
- Ollama: `http://127.0.0.1:11434`, native `GET /api/tags`, `POST /api/chat`. James does not depend on Ollama's optional OpenAI shim.
- llama.cpp server: default port 8080, OpenAI-compatible endpoints when the runtime supports them.
- Other OpenAI-compatible server: user-selected **numeric loopback port** and compatible `/v1` endpoints only.

Model IDs and presence in a list are not proof of successful inference, tool calling, vision support or context length. JSON fixtures test request/response contracts; run `scripts/test-local-provider.ps1` with the server and model on **your machine** for actual inference. If a provider does not implement the expected compatible routes, James reports an error rather than silently switching providers. Remote URLs, API keys, cloud fallback and network exposure are deliberately not implemented.

**Currently implemented:** non-streaming text requests, optional explicit PNG attachment for compatible vision models, bounded responses and timeouts, worker-thread UI offloading, session persistence. **Not implemented:** token streaming, network-level abortion of a dispatched request, tool calling, model capability negotiation, authenticated remote inference, secure credential storage. Stop discards cancelled responses but the server may continue processing.

## Desktop capabilities

The Rust desktop application can compile on platforms where its dependency stack is available. Screen capture, Guide me, spoken answers and listening (Talk, Dictate) are implemented on Windows, macOS and Linux: macOS/Linux use standard system tools resolved from fixed system directories (never a shell), and listening there runs a separately installed whisper.cpp on the device, with temporary files confined to a private folder deleted immediately. Settings reports whether listening is ready and what is missing. Computer input returns `Unsupported` outside Windows, and on macOS and Linux the UI explains this limitation rather than presenting functional input controls. These macOS/Linux capabilities are covered by unit tests and CI, not yet by real-device acceptance tests. On Windows, action execution is bound to a fresh screenshot, exact proposed action, short approval window, one-use grant and durable outstanding-action marker. It does **not** guarantee target-app focus, OS sandboxing or outcome verification. No platform bypasses OS prompts.

Local session JSON is stored in `%LOCALAPPDATA%/Vers3Dynamics/James` on Windows or `~/.james` when HOME exists. Action intents contain potentially sensitive typed text until acknowledged. See [SECURITY.md](../SECURITY.md).

## Archive meaning

CI may upload a portable Windows executable, Windows per-user installer, and, after successful host builds, tarball archives for the GitHub-hosted macOS/Linux runner architectures. These archives are **unsigned** experimental outputs, not universal installers. GitHub-hosted macOS binaries may be blocked by Gatekeeper; no code-signing/notarization claim is made. Linux binaries can require compatible graphics libraries and display server. Never advertise an architecture until its actual artifact and runner architecture are verified.

See [RELEASE_READINESS.md](RELEASE_READINESS.md) and [ACCEPTANCE_TESTS.md](ACCEPTANCE_TESTS.md) before public distribution.
