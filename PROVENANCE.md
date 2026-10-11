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

The Talk loop, Guide me, speech text cleanup, LM Studio/Ollama/llama.cpp presets, and the macOS/Linux capture and speech helpers were independently authored for James on 2026-10-11 from the project's own requirements and the public documentation of the system tools they call (`screencapture`, `say`, `grim`, `gnome-screenshot`, `spectacle`, ImageMagick, `espeak-ng`, `spd-say`). No reference product code, prompts, binaries or assets were used, and no new Rust dependencies were added.

whisper.cpp listening (the endpointer, WAV encoding, transcript cleanup, recorder fallback and Settings setup) was likewise independently authored on 2026-10-11. It invokes a separately installed [whisper.cpp](https://github.com/ggml-org/whisper.cpp) (MIT) `whisper-cli` and system audio recorders as external programs; neither whisper.cpp nor any model weights are vendored, linked or redistributed, and James does not download models.

The Lab client implements the public `lop-nur-twin` repository's actual HTTP contracts (source commit and locations in [LAB.md](docs/LAB.md)). Direct Rust dependencies are eframe/egui (MIT OR Apache-2.0), image (MIT OR Apache-2.0), serde/serde_json (MIT OR Apache-2.0), base64 (MIT OR Apache-2.0), and ureq 2 (MIT OR Apache-2.0). Cargo.lock records transitive package versions; this list is not a complete transitive license audit. Fixed Windows helpers use installed Windows components. Inno Setup is a build-time compiler and is not bundled as a compiler with James. LM Studio, model weights and R.A.I.N. Lab are not bundled.
