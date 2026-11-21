# Source provenance

## Floating companion sculpture (2026-11-21)

The desktop octopus, hourglass and bell were modeled for James in Blender.
The procedural source defines the eight arm paths, skin, eyes, stoneware glaze,
props, lighting and greeting poses. It produces the WebP sheets bundled in
`crates/james-desktop/assets/octopus`. The sculpt and renders share the
repository's MIT license. [Blender source and rebuild command](scripts/render-companion.py).

## November 2026 Team and agent-interop/v1

`james-agents`, `james-interop`, the Team page and the Lab interop client
were written for James on 16–17 November 2026 by Claude Code (Anthropic),
working in this repository. Ideas were taken, not code, from
[SuperBot](https://github.com/davidondrej/superbot) (MIT, © 2026 David
Ondrej) at commit `652e100`: one small adapter per model source, routines as
data, delegation as a message to another assistant's queue, queued work that
survives a restart, and interrupted work reported rather than replayed. Its
permission-bypass flags, single all-powerful token, unbounded delegation and
self-edited memory were deliberately not adopted. No SuperBot source, text,
images or names were copied, so its licence imposes no notice here; this
entry records the influence. The `agent-interop/v1` specification and its
fixtures are shared with `topherchris420/lop-nur-twin`, whose own code wrote
the fixtures. The demonstration transcripts in `docs/team-demo/` are real
runs: the scripted team, and Qwen2.5-1.5B/3B-Instruct Q4_K_M GGUF files from
Hugging Face served by llama.cpp `e8fbcad` on a CPU container. No model files
are in this repository.

## November 2026 README campaign artwork

The two images in [`docs/assets/marketing`](docs/assets/marketing/README.md)
were generated for James with OpenAI's built-in image-generation tool on
2026-11-15 and encoded as WebP for the README. They are conceptual campaign
illustrations, not application screenshots or evidence of implemented UI.
The octopus artwork interprets the existing companion's lavender palette;
the glass layers illustrate the theme of inspectable context. No Apple logo,
product image, interface, or other third-party visual asset was incorporated.
The artwork is supplied under this repository's MIT license; the prompts and
asset inventory are recorded alongside it. Existing application screenshots
remain unchanged and retain their recorded validation provenance below.

## November 2026 desktop feature provenance

The November frontend/backend updates introduced elapsed history groups,
current-chat navigation, configurable guidance marks, strict coordinate
parsing, a user-confirmed guidance checklist, multiple attachments,
live voice follow-ups, advisory step checks, reviewed typing Undo and the
octopus companion. The desktop behavior and its regression tests were
implemented in James's Rust code. Some functional requirements were informed
by public product information and earlier external product research.
This is not a claim of formally segregated clean-room development.
No third-party application source, prompts, interface artwork or assets
were incorporated or redistributed.

Codex and additional model-assisted implementation passes contributed
to these changes under project-scoped file access. `global-hotkey` 0.10
(MIT OR Apache-2.0) adds shortcut registration, test-only `base64` 0.22
(MIT OR Apache-2.0) inspects image payloads, and `pdf-extract` 0.12.1
(MIT) supports text extraction from PDFs. The existing `image` dependency
enables JPEG/WebP features. Cargo.lock and generated third-party notices
record the dependency resolutions.

The history archive, companion, overlays, voice interactions, Windows
input review, file workflows, media widget and platform integrations were
written for James. Windows input handling uses documented operating-system
APIs, and model outputs do not become unreviewed executable instructions.
Claims of correctness or originality must still be evaluated against
the project's source and tests. Historical commits remain available for
source and implementation provenance.

## v0.3 desktop integrations

The new platform helpers, model vision request serialization, review UI,
Lab client and installer were authored for James using documented platform
and provider APIs. No third-party application binaries or proprietary
assets are shipped with the project.

## Voice conversation, Guide me and cross-platform helpers

The Talk loop, Guide me, speech text cleanup, and the macOS/Linux capture and speech helpers were independently authored for James on 2026-10-11 from the project's own requirements and the public documentation of the system tools they call (`screencapture`, `say`, `grim`, `gnome-screenshot`, `spectacle`, ImageMagick, `espeak-ng`, `spd-say`). No reference product code, prompts, binaries or assets were used, and no new Rust dependencies were added.

whisper.cpp listening (the endpointer, WAV encoding, transcript cleanup, recorder fallback and Settings setup) was likewise independently authored on 2026-10-11. It invokes a separately installed [whisper.cpp](https://github.com/ggml-org/whisper.cpp) (MIT) `whisper-cli` and system audio recorders as external programs; neither whisper.cpp nor any model weights are vendored, linked or redistributed, and James does not download models.

The app icon (a teal "J" and dot on a dark rounded square) is original geometric artwork drawn by [`scripts/make-icons.py`](scripts/make-icons.py) with Pillow on 2026-10-12; no font, template or third-party image was used. The packaging scripts, Linux launcher and installer, macOS `Info.plist` and issue form were independently authored for James on the same date. The Linux container build downloads the official `rustup` installer and Ubuntu packages at build time; none of them are redistributed except as the `.deb`'s declared dependencies.

The Lab client implements the public `lop-nur-twin` repository's actual HTTP contracts (source commit and locations in [LAB.md](docs/LAB.md)). Direct Rust dependencies are eframe/egui (MIT OR Apache-2.0), image (MIT OR Apache-2.0), serde/serde_json (MIT OR Apache-2.0), base64 (MIT OR Apache-2.0), and, through the desktop, their transitive dependencies. The HTTP transport is James's own `james-loopback` crate, written for this project on 2026-10-12 from RFC 9112's message framing rules; it uses only the Rust standard library and replaced ureq 2. Cargo.lock records transitive package versions; this list is not a complete transitive license audit. Fixed Windows helpers use installed Windows components. Inno Setup is a build-time compiler and is not bundled as a compiler with James. LM Studio, model weights and R.A.I.N. Lab are not bundled.

## 0.4 end-user release

The guided setup, model capability probes and error classes, the Markdown renderer, themes, session and privacy handling, the richer action record and recovery screen, the Windows target-window check, the Lab summaries, the GUI tests, the probe example, `scripts/check-powershell.ps1`, the AppStream metadata, the uninstall prompt and the signing and provenance steps were independently authored for James on 2026-10-12, from the project's requirements and the public documentation of the APIs and tools involved (Ollama, llama.cpp and LM Studio HTTP APIs, AppStream, Inno Setup, `signtool`, `notarytool`, GitHub artifact attestations). The Markdown renderer is James's own; no Markdown library was added.

New dependencies: eframe's `accesskit` feature, which brings in the AccessKit crates (MIT OR Apache-2.0; `accesskit_winit` is Apache-2.0) and, on Linux, the zbus D-Bus stack (MIT) for AT-SPI; and, for tests only, `egui_kittest` 0.31.1 and `kittest` 0.1.0 (MIT OR Apache-2.0), which are not part of any package. Every package now ships `THIRD-PARTY-NOTICES.txt`, generated by `tools/notices` (written for James on 2026-10-12) from `Cargo.lock` and the license files each crate publishes. The standard texts in `installer/licenses`, used for the few crates that publish no license file, are copied unchanged from the SPDX License List data v3.29.0 (CC0-1.0). The notices cover Rust crates only; the Windows build also links Microsoft's C runtime from Visual Studio statically, which they do not describe.

Real servers and models were used for testing only and are not in the repository or the packages: Ollama (MIT) and llama.cpp (MIT) built from their public source tags, and model weights downloaded from their publishers (SmolVLM2-500M-Video-Instruct and Qwen2.5 under Apache-2.0, Gemma 3 under Google's Gemma terms, nomic-embed-text under Apache-2.0). Test fixtures repeat short response shapes and error messages those servers returned through their public APIs.

The screenshots in `docs/assets` are unedited captures of James itself, taken in this release's test sessions; the conversation shown is a real answer from Gemma 3 4B served by Ollama. The Lab client follows `lop-nur-twin` at commit `4871ef6` (see [LAB.md](docs/LAB.md)); no Lab code is copied into James.

## Capture self-exclusion

The Windows screenshot flow waits for James's own window to leave the
picture before capturing. Its implementation uses documented Win32
functions, including `EnumWindows`, `IsWindowVisible`, `IsIconic`
and `GetWindowRect`. The desktop release-notes feature displays James's
own changelog, and Guide me renders original marks on an explicitly
captured screenshot. No third-party app assets are shipped by these features.
