# Platform support

James 0.4.0 is a **developer preview** (see [END_USER_READINESS.md](END_USER_READINESS.md)).
This page says, for every capability and platform, how far the claim has been
checked. A green CI run proves code compiles and its tests pass on that
runner; it does not prove a person can install and use James on that system.

## How to read the matrix

| Mark | Meaning |
| --- | --- |
| **Real** | Exercised on that operating system with real software (a real model server, a real package manager) and recorded in [VALIDATION.md](VALIDATION.md). Says where it ran. |
| **Auto** | Implemented and covered by automated tests that run in CI on that operating system. No person used it there. |
| **Impl** | Implemented and compiled for that platform, with tests elsewhere, but not run there. |
| **No** | Not implemented on that platform. The interface says so instead of showing working controls. |

"Real (container)" means a Linux container or VM with a virtual display
(Xvfb, software OpenGL) and no window manager: real packages, real model
servers and the real app, but not a physical desktop, GPU driver, Wayland
session, microphone or screen reader.

## Capability matrix

| Capability | Windows 10/11 x64 | macOS 11+ (Apple silicon, Intel) | Linux x64 (Ubuntu 22.04+, Debian 12+, Mint 21+) |
| --- | --- | --- | --- |
| Package builds (installer/portable, universal app, .deb/archive) | Auto (CI builds the installer and portable `.exe`) | Auto (CI builds both slices, assembles and ad-hoc signs `James.app`, runs `--version` on arm64) | Real (container): `.deb` built on 22.04 in CI and installed there; installed, upgraded from 0.3.0 and removed on 24.04 |
| Install, first launch, upgrade, uninstall | Impl. Uninstaller asks whether to delete data. Never run by a person | Impl. Never run by a person; Gatekeeper prompt unrecorded | Real (container): apt install, upgrade 0.3.0 → 0.4.0 keeping settings and history, apt remove keeping `~/.james`; archive `install.sh` and `--uninstall` |
| Guided first-run setup | Auto (GUI tests in CI) | Auto (GUI tests in CI) | Real (container), with Ollama 0.40.3 and llama.cpp b11564 |
| Find my model server, model capabilities, cloud-model warning | Auto | Auto | Real (container) against Ollama and llama.cpp |
| Chat with streaming, Stop, Ask again | Auto | Auto | Real (container) with Ollama `gemma3:4b`; Stop made Ollama cancel the generation |
| Markdown replies, themes, high contrast, text size, reduced motion | Auto | Auto | Real (container), dark and light |
| Keyboard navigation and shortcuts | Auto | Auto | Real (container) with X11 key events; Auto |
| Screen-reader support (AccessKit) | Impl (UI Automation) | Impl (NSAccessibility) | Impl (AT-SPI). No screen reader has been used on any platform |
| Screen capture | Auto: CI runs a capture on a Windows runner session | Impl (`screencapture`; needs Screen Recording permission) | Real (container): ImageMagick `import` on X11. gnome-screenshot, spectacle and grim (Wayland) not run |
| Screen questions and Guide me | Auto | Auto | Real (container): Gemma 3 4B named on-screen labels correctly; SmolVLM2 500M did not |
| Reviewed input (one click, key or text) | Auto: review, deny and countdown cancel are GUI-tested; CI checks the helper refuses when the reviewed window is not in front. **No real input has been sent in 0.4.0 testing** | No (explains instead) | No (explains instead) |
| Target-window check before input | Auto (see above) | No input | No input |
| Crash recovery of an unchecked action | Auto | Auto | Real (container): a 0.3-format record was shown after upgrade and cleared only by the person |
| Talk, Dictate, read aloud | Impl (Windows speech). Not run in 0.4.0 testing | Impl (whisper.cpp, `say`) | Impl (whisper.cpp, espeak-ng / spd-say). No microphone in the test environment |
| R.A.I.N. Lab view | Auto | Auto | Real (container) against lop-nur-twin 4871ef6 |
| Owner-only data files | Default `%LOCALAPPDATA%` permissions (not changed or checked by James) | Auto (0700 folder, 0600 files) | Real (container) and Auto |
| Code signing | **Unsigned** (Authenticode only if a maintainer configures a certificate) | **Unsigned**, ad-hoc only (Developer ID and notarization only if configured) | Not applicable (checksums and provenance attestations) |

### Not supported

| Platform | Status |
| --- | --- |
| Windows on ARM (native) | No native build. The x64 build may run under emulation; untested |
| Linux on ARM64 | The packaging script can build it, but CI does not and it is untested |
| Wayland-native screen capture | `grim` is supported in code (wlroots desktops); GNOME and KDE Wayland portals are not |
| Browser, Android, iOS | Not available |

## Local model servers

| Server | Protocol James uses | Tested with |
| --- | --- | --- |
| Ollama | `/api/tags`, `/api/show`, `/api/ps`, `/api/chat` (NDJSON) | **Real:** Ollama built from source tag v0.40.3 (it reports 0.0.0 because a source build has no release stamp), `qwen2.5:0.5b`, `gemma3:4b`, `nomic-embed-text`, `gpt-oss:20b-cloud` |
| llama.cpp `llama-server` | `/v1/models`, `/props`, `/v1/chat/completions` (SSE) | **Real:** built from tag b11564 with SmolVLM2-500M (vision) and Qwen2.5-1.5B (text) |
| LM Studio | `/v1/models`, `/api/v0/models`, `/v1/chat/completions` | **Fixtures only.** Not run against a real LM Studio |
| Other OpenAI-compatible servers | `/v1/models`, `/v1/chat/completions` | llama-cpp-python 0.3.36 in an earlier session; fixtures |

What James reports about a model (reads images, chat, context window,
loaded) always comes from that server; when a server says nothing, James
says it doesn't know. A loopback address proves the *connection* stays on
this computer, not that the *model* does: Ollama, for one, can relay to its
cloud service. James labels the models a server reports as remote and never
selects one automatically, but other servers may relay without saying so.

## Packages

| File | For | Notes |
| --- | --- | --- |
| `James-<version>-windows-x64-setup.exe` | Windows 10/11 x64 | Per-user install to `%LOCALAPPDATA%\Programs\James`, Start menu entry, no administrator prompt, static C runtime |
| `James-<version>-windows-x64-portable.exe` | Windows 10/11 x64 | Same program, no installation |
| `James-<version>-macos-universal.zip` | macOS 11+ | Universal `James.app` (arm64 + x86_64) |
| `james_<version>_amd64.deb` | Debian-based x64 | Requires glibc 2.35+, OpenGL and `libxkbcommon-x11-0`; includes AppStream metadata |
| `James-<version>-linux-x86_64.tar.gz` | Other x64 Linux | `install.sh` installs for the current user |

See [INSTALLER.md](INSTALLER.md) for how each is built and released.
