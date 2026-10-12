# Source provenance

James's Rust source, tests, demonstration, architecture, and project documents were authored independently for this project on 2026-10-11. Functional requirements were informed by public Korso documentation and bounded REA inspection of its publicly downloadable Windows package.

Research and implementation occurred in the same investigation. This is an independent implementation, not a claim of a formally segregated clean-room process. No reference code, source maps, prompts, UI layout, logos, fonts, images, sounds, or pet assets were copied into James. No Korso backend or private repository was accessed, and no Korso executable was launched. Public website JavaScript did run in an isolated browser during declared REA page captures.

The reference installer, extracted files, and complete REA evidence remain outside the James repository. Their identifiers and hashes are recorded in [RESEARCH.md](docs/RESEARCH.md); James's MIT license does not license those artifacts. Public research documentation contains concise findings and provenance, not recovered reference source.

`james-core` currently depends only on the Rust standard library. There are no vendored third-party runtime packages or visual assets. The CI workflow uses GitHub's checkout action at a pinned commit; the project neither redistributes nor relicenses that action. Add a license/dependency inventory before introducing external crates or assets.

Architectural recommendations about eframe, iced, Tauri, Tokio, SQLite, inference servers, and MCP are sourced to their primary documentation in [ARCHITECTURE.md](docs/ARCHITECTURE.md). Proposed components are not already integrated.

Contributions must identify the origin and license of new code and assets. Keep proprietary research files outside the repository, preserve observations versus inferences, and never replace benchmark evidence with marketing claims.


## v0.3 desktop integrations

The new platform helpers, model vision request serialization, review UI, Lab client and installer were independently authored for James. REA inspected the public Korso landing page on 2026-10-11; the bounded observation and design mapping are recorded in [DESKTOP-RESEARCH.md](docs/DESKTOP-RESEARCH.md). No Korso code, assets, private API or recovered prompts are shipped. Complete page evidence is retained outside the repository.

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

The wait for James to leave the picture before a Windows screenshot was independently authored for James on 2026-10-12. Its requirement came from REA-assisted inspection of the Korso 0.7.8 Windows installer (`Korso-windows-x64-setup.exe`, SHA-256 `a44f6effcb604ec2f5e8b4296338d0170b17884a402c86cfc8b1ed02792cf8a5`), whose capture path hides its own windows, waits for composition and only then captures. The installer was extracted, not run. No Korso code was copied; the timing values were chosen for James, and the window checks use the documented Win32 functions `EnumWindows`, `IsWindowVisible`, `IsIconic` and `GetWindowRect`. Korso's preferred route, `SetWindowDisplayAffinity`, only affects windows of the calling process, so James can't use it without unsafe code, which the workspace forbids.
