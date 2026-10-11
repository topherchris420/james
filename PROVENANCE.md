# Source provenance

James's Rust source, tests, demonstration, architecture, and project documents were authored independently for this project on 2026-10-11. Functional requirements were informed by public Korso documentation and bounded REA inspection of its publicly downloadable Windows package.

Research and implementation occurred in the same investigation. This is an independent implementation, not a claim of a formally segregated clean-room process. No reference code, source maps, prompts, UI layout, logos, fonts, images, sounds, or pet assets were copied into James. No Korso backend or private repository was accessed, and no Korso executable was launched. Public website JavaScript did run in an isolated browser during declared REA page captures.

The reference installer, extracted files, and complete REA evidence remain outside the James repository. Their identifiers and hashes are recorded in [RESEARCH.md](docs/RESEARCH.md); James's MIT license does not license those artifacts. Public research documentation contains concise findings and provenance, not recovered reference source.

`james-core` currently depends only on the Rust standard library. There are no vendored third-party runtime packages or visual assets. The CI workflow uses GitHub's checkout action at a pinned commit; the project neither redistributes nor relicenses that action. Add a license/dependency inventory before introducing external crates or assets.

Architectural recommendations about eframe, iced, Tauri, Tokio, SQLite, inference servers, and MCP are sourced to their primary documentation in [ARCHITECTURE.md](docs/ARCHITECTURE.md). Proposed components are not already integrated.

Contributions must identify the origin and license of new code and assets. Keep proprietary research files outside the repository, preserve observations versus inferences, and never replace benchmark evidence with marketing claims.
