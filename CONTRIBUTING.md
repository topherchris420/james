# Contributing

Start with [README.md](README.md), the [architecture](docs/ARCHITECTURE.md), and the [roadmap](docs/ROADMAP.md). Phase 1 is an executable contract foundation. Keep changes small enough to review and state which integration, if any, they actually implement.

Run the four checks in the README before proposing a change. Add regression tests for authorization, recovery, parsing, cancellation, or budget behavior affected by your change. Formatting-only changes do not need new tests.

Models and extension metadata are untrusted inputs. Only the host registry classifies effects and validates resource scope. A passing in-memory contract test does not prove OS confinement or durable execution. Include adversarial fixtures and failure-path evidence when implementing those boundaries.

Keep reference source, binaries, captured private data, credentials, and assets out of commits. Contributions to original James files are under the MIT license. Document the provenance and license of any third-party addition.

PR descriptions should explain the concrete behavior change, relevant tests, supported platforms, and remaining limitations. Update public status statements when an integration becomes real. Do not claim comparative advantages without results following [BENCHMARKS.md](docs/BENCHMARKS.md).
