# Contributing

Start with [README.md](README.md), the [architecture](docs/ARCHITECTURE.md), and the [roadmap](docs/ROADMAP.md). The repository contains an executable contract foundation and a native desktop preview with streamed local-model chat, screen context, voice and reviewed Windows input. All network access goes through `crates/james-loopback`; do not add another HTTP client. Keep changes small enough to review and state which integration, if any, they actually implement.

Run the README formatting, tests, strict Clippy and Rust examples, plus model/desktop builds when touching those crates. When changing packaging, run `scripts/package-linux.sh` (or `scripts/package-macos.sh` on a Mac) and see [INSTALLER.md](docs/INSTALLER.md); user-facing wording belongs in [GETTING_STARTED.md](docs/GETTING_STARTED.md) as well as the app. First-time workspace builds need Cargo registry access. Add regression tests for authorization, journal failure, recovery, parsing, cancellation, or budget behavior affected by your change. Formatting-only changes do not need new tests.

Models and extension metadata are untrusted inputs. Only the host registry classifies effects and validates resource scope. A passing in-memory contract test does not prove OS confinement or durable execution. Include adversarial fixtures and failure-path evidence when implementing those boundaries.

Keep reference source, binaries, captured private data, credentials, and assets out of commits. Contributions to original James files are under the MIT license. Document the provenance and license of any third-party addition.

PR descriptions should explain the concrete behavior change, relevant tests, supported platforms, and remaining limitations. Update public status statements when an integration becomes real. Do not claim comparative advantages without results following [BENCHMARKS.md](docs/BENCHMARKS.md).
