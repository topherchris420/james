# James platform support and distribution

Status: **developer previews, not a universal production release**. A successful compilation, CI run, or archived binary is not a clean-machine installation, accessibility or native-permissions validation.

| Platform / architecture | Build pipeline | Native screen / input / speech | End-user status |
| --- | --- | --- | --- |
| Windows 10/11 x64 | Per-user installer and portable `.exe` from GitHub Actions, with tests | Windows-only fixed helpers; approved one action at a time | Preview; installer lifecycle and real-model integration still require hands-on verification |
| Windows 11 ARM64 | No native ARM64 build established | x64 compatibility not validated on all devices | Unsupported as a native target; compatibility is unverified |
| macOS 11+ Apple silicon | Universal `James.app` (zip) built on the arm64 macOS runner, ad-hoc signed, started with `--version` in CI | Screen capture (`screencapture`), speech (`say`) and whisper.cpp listening implemented, not device-validated. No computer input | Preview; unsigned with a Developer ID and unnotarized, so first launch needs **Open Anyway**. No hands-on Mac run recorded |
| macOS 11+ Intel | The x86_64 slice of the same universal app; cross-compiled, never run in CI | As above | Unverified on Intel hardware |
| Linux x64 (glibc 2.35+) | `.deb` and portable archive built in an Ubuntu 22.04 container; both installed and started with `--version` there in CI | Screen capture (gnome-screenshot, spectacle, grim, ImageMagick), speech (espeak-ng, spd-say) and whisper.cpp listening implemented, not device-validated. No X11/Wayland input or portal capture adapter | Preview; the `.deb` installed with apt, its window opened (Xvfb, software OpenGL) and removed on an Ubuntu 24.04 VM. Display server and system library dependencies vary |
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

**Currently implemented:** streamed text replies (server-sent events for OpenAI-compatible servers, NDJSON for Ollama, with a fallback for servers that answer in one piece), optional explicit PNG attachment for compatible vision models, bounded responses and idle timeouts, a Stop that closes the connection, worker-thread UI offloading, session persistence. **Not implemented:** tool calling, model capability negotiation, authenticated remote inference, secure credential storage. Closing the connection does not prove the server stopped generating; that is the server's behavior.

## Desktop capabilities

The Rust desktop application can compile on platforms where its dependency stack is available. Screen capture, Guide me, spoken answers and listening (Talk, Dictate) are implemented on Windows, macOS and Linux: macOS/Linux use standard system tools resolved from fixed system directories (never a shell), and listening there runs a separately installed whisper.cpp on the device, with temporary files confined to a private folder deleted immediately. Settings reports whether listening is ready and what is missing. Computer input returns `Unsupported` outside Windows, and on macOS and Linux the UI explains this limitation rather than presenting functional input controls. These macOS/Linux capabilities are covered by unit tests and CI, not yet by real-device acceptance tests. On Windows, action execution is bound to a fresh screenshot, exact proposed action, short approval window, one-use grant and durable outstanding-action marker. It does **not** guarantee target-app focus, OS sandboxing or outcome verification. No platform bypasses OS prompts.

Local session JSON is stored in `%LOCALAPPDATA%/Vers3Dynamics/James` on Windows or `~/.james` when HOME exists. Action intents contain potentially sensitive typed text until acknowledged. See [SECURITY.md](../SECURITY.md).

## Package meaning

Tagged releases and CI artifacts contain a Windows per-user installer and portable executable, a universal macOS `James.app`, and a Linux `.deb` and portable archive (see [INSTALLER.md](INSTALLER.md)). All are **unsigned** preview builds: Windows SmartScreen and macOS Gatekeeper warn on first launch, and the [getting started guide](GETTING_STARTED.md) explains how to proceed. The Linux binary is built against glibc 2.35 so it starts on Ubuntu 22.04, Debian 12, Linux Mint 21 and newer; it still needs an OpenGL driver, `libxkbcommon-x11` and a display server. CI starting a binary with `--version` proves it loads on that system, not that its window, permissions or native features work. Never advertise an architecture until its actual artifact has been run there.

See [RELEASE_READINESS.md](RELEASE_READINESS.md) and [ACCEPTANCE_TESTS.md](ACCEPTANCE_TESTS.md) before public distribution.
