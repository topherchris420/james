# James release readiness

**Status: not production ready.** This page is a release gate, not an assertion of completed hardware validation.

## Implemented in cross-platform foundation work

- Existing: Rust workspace with core one-use approval policies, Windows screenshot/speech/input helper, action-intent crash gate, optional read-only R.A.I.N. Lab discovery bridge, streamed OpenAI-compatible and Ollama loopback chat through the dependency-free `james-loopback` client, Windows preview installer, and CI tests.
- Added: explicit user-selected provider protocol and persisted provider choice (legacy sessions default to LM Studio), native Ollama model listing/text chat and PNG request encoding, llama.cpp/OpenAI-compatible presets, UI messages about unsupported native capabilities, CI macOS desktop build and experimental Unix portable packaging, and `scripts/test-local-provider.ps1`.
- Important: adding compatible endpoints does not constitute a real-model inference benchmark. The tested protocols are explicitly scoped to local loopback and do not expose unauthenticated services.

## Gates, not marketing claims

| Release gate | State / evidence required |
| --- | --- |
| Workspace formatting, Clippy, tests | Must pass on the target commit in GitHub Actions |
| Windows x64 optimized executable and Inno Setup installer | Require successful artifact jobs and checksum verification |
| macOS universal `James.app` | CI builds both slices, signs ad hoc and runs the arm64 slice with `--version`; launch, Gatekeeper and permission prompts on real Apple silicon and Intel Macs still need recording |
| Linux `.deb` and portable archive | CI builds them on Ubuntu 22.04, installs both and runs `--version` there; the `.deb` window has opened on Ubuntu 24.04 under Xvfb. Real desktops (X11 and Wayland, GPU drivers) still need recording |
| Windows ARM64, Linux ARM64 and other macOS architecture | No independent device validation yet |
| Actual LM Studio/Qwen text and vision | Run and record on a real loaded model. This environment cannot assume access to the user's own localhost |
| Ollama/llama.cpp genuine server responses | Native fixtures added; run real servers and record failures before compatibility claims |
| Streaming and cancellation | Implemented at the transport level: Stop closes the socket. Socket fixtures and a scripted-server UI session pass; real LM Studio, Ollama and llama.cpp streaming still need recorded runs |
| SQLite recovery and encrypted credentials | **Not implemented**; keep existing session backwards compatible |
| Native macOS/Linux screen capture, speech and whisper.cpp listening | Implemented with unit tests; real-device validation (permissions, microphones, recognition quality, display servers) required |
| Native macOS/Linux computer input | **Not implemented**; explicitly disabled in UI |
| Voice conversation (Talk) and Guide me | Implemented; hands-on conversation, latency and guidance-quality tests with real models required |
| First-run setup | **Find my model server** and its no-server, no-model and found outcomes exercised through the real UI against scripted servers; needs a run against real LM Studio and Ollama installs |
| User experience/accessibility | Human test with keyboard, screen reader, high contrast, DPI scaling, reduced motion, initial setup |
| Installer upgrade/uninstall | Clean-machine tests including retained history and unresolved-action recovery |
| PWA and mobile companion | Not implemented; independent design/security validation required |
| Signed Windows and notarized macOS distribution | Code-signing certificate, Apple Developer ID and actual notarization required |
| Privacy and independent security review | Threat model, action safety, current-process focus, persistence, provider logging and sensitive text retention |

Use [ACCEPTANCE_TESTS.md](ACCEPTANCE_TESTS.md) to capture real results instead of inventing benchmarks. Do not check off a gate based solely on Rust compilation or an endpoint fixture. The release workflow only publishes on an explicit `v*` tag after its jobs pass; neither this document nor a pushed branch creates a signed public release.

## Release decision

An initial *developer preview* can be shared with prominently stated limitations after all relevant CI passes. A claim that anyone can safely install and use James on every supported desktop requires reproducible install, actual-model inference, privacy, accessibility and OS-native permission/control validation on every advertised architecture. Human review remains mandatory: **intelligence proposes, evidence is checked, humans decide**.
