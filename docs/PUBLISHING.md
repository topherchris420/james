# Publication preparation

The Phase 1 source repository is [topherchris420/james](https://github.com/topherchris420/james), with `main` as the intended default branch. It contains research, architecture, and a tested core contract prototype. The crate remains `publish = false`; there is no crates.io release or usable desktop release.

## Prepared

- Original Rust workspace with lockfile, in-memory authorization/supervisor contracts, adversarial tests, and two no-I/O demonstrations.
- MIT license, contribution guide, provenance statement, and explicit security scope.
- Evidence ledger, ADRs, requirements, roadmap, benchmark plan, and validation record.
- Git ignore rules exclude build output, proprietary binary formats, common credential files, and private research directories.
- CI workflow checks stable and the declared minimum Rust version on Windows, Linux, and macOS. Checkout is pinned to a resolved commit and credentials are not persisted. Hosted checks passed on all six OS/toolchain combinations after the supervised-core upgrade; see [validation](VALIDATION.md).

## Phase 1 publication checks

1. Owner/repository: `topherchris420/james`. Description identifies Phase 1 research, architecture, and the authorization core. The owner maintains the security channel; private vulnerability reporting was enabled and verified on 2026-10-11.
2. The initial file list and source provenance were reviewed. The core has no third-party dependencies. Research artifacts and raw captures remain outside the repository; a common credential-pattern scan found no matches.
3. README explicitly identifies Phase 1 status. Desktop downloads, model support, and sandbox guarantees remain future work.
4. Initial local formatting, 26 contract tests, strict Clippy, and the demonstration passed before the first push. The update adds seven supervisor contract tests and a second simulated demonstration. Hosted CI has passed on Windows, Linux and macOS with stable and Rust 1.85.0; this is compile/test evidence, not a desktop-runtime release gate. Pull request #1 was reviewed through CI and merged to `main`; repository-wide branch protection is a separate administration decision.
5. Public findings retain source links and incomplete-analysis qualifications. The proprietary reference bundle is not included.

## Before a usable desktop release

Complete the roadmap's end-to-end, recovery, privacy, accessibility, packaging/update, and platform gates. Generate a dependency/license inventory and a signed release manifest containing artifact hashes. Configure release ownership and supported-version policy. Publish benchmark methodology and raw synthetic-task results before asserting any comparative advantage.

The repository license does not authorize redistribution of reference artifacts. James's scope and claims should reflect its actual released implementation.
