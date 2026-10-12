# James release readiness

**Status for 0.4.0: developer preview, not production ready.**

The scored release gate, its hard gates and the release classification are in
[END_USER_READINESS.md](END_USER_READINESS.md). How far each feature has been
checked on each system is in [PLATFORM_SUPPORT.md](PLATFORM_SUPPORT.md), the
scenarios still to run are in [ACCEPTANCE_TESTS.md](ACCEPTANCE_TESTS.md), and
the evidence itself is in [VALIDATION.md](VALIDATION.md). This page is the
short checklist.

## Gates

| Release gate | State in 0.4.0 |
| --- | --- |
| Formatting, Clippy and tests | Pass in CI on Windows, macOS and Ubuntu with stable Rust; the library crates also with Rust 1.85 and the desktop with 1.88 |
| Windows installer and portable `.exe` | Built and checksummed in CI, including the uninstaller's keep-or-delete prompt. Never installed or launched by a person |
| macOS universal `James.app` | Built from both slices, ad-hoc signed, arm64 slice runs `--version` in CI. Launch, Gatekeeper and permission prompts on real Macs not recorded |
| Linux `.deb` and archive | Built on Ubuntu 22.04, validated (`desktop-file-validate`, `appstreamcli`, `lintian`) and installed there; upgrade from 0.3.0, launch and removal checked on Ubuntu 24.04 in a container. No physical desktop |
| Windows ARM64, Linux ARM64 | Not built in CI, not tested |
| Ollama and llama.cpp | Real servers (0.40.3, b11564): chat, streaming, Stop, vision, capabilities, error classes |
| LM Studio | Fixtures only; **needs a real run** |
| First-run setup | Real Ollama and llama.cpp in a container; GUI tests on all three CI systems |
| Screen capture, speech and whisper.cpp on macOS and Linux | Implemented; Linux X11 capture run in a container; no microphone, Wayland or Mac tested |
| Computer input | Windows only. Review, Deny and countdown Cancel GUI-tested on the Windows runner, where the helper also refuses a wrong target window. **No real input sent in 0.4.0 testing.** macOS and Linux: not implemented, explained in the app |
| Recovery of an unchecked action | Automated; a 0.3 record recovered after a real Linux upgrade. No real Windows crash |
| Accessibility | AccessKit enabled; keyboard, contrast and 1x-2x/100-200% layouts tested automatically. **No screen reader or human test** |
| Data protection | Owner-only files and history controls verified on Linux; not encrypted; Windows permissions unchecked |
| Signed Windows and notarized macOS builds | Supported by the pipeline when certificates are configured; **none exist** |
| Third-party license notices in packages | **Missing**: packages ship James's MIT license only |
| Independent security review | **Not done** |
| SQLite history, encrypted credentials, PWA or mobile companion | Not implemented |

Do not check off a gate on the strength of compilation or a fixture. The
release workflow publishes only on an explicit `v*` tag after its jobs pass;
neither this page nor a pushed branch creates a signed public release.

## Release decision

0.4.0 can be shared as a **developer preview** with its limitations stated
prominently, which the README, the release notes and the app's own wording
do. A public beta needs the real-hardware runs listed under "What would raise
the score" in [END_USER_READINESS.md](END_USER_READINESS.md); anything beyond
that also needs signing, a security review, screen-reader testing and a study
with first-time users. Human review remains mandatory: **intelligence
proposes, evidence is checked, humans decide**.
