# Publication preparation

The [James repository](https://github.com/topherchris420/james) now contains the original authorization core and a working desktop text-chat preview. Crates remain `publish = false`; there is no crates.io release or signed desktop installer. A portable Windows binary is published as a temporary [CI artifact](https://github.com/topherchris420/james/actions/workflows/desktop.yml).

## Prepared

- Original Rust workspace with lockfile, in-memory authorization/supervisor contracts, adversarial tests, and two no-I/O demonstrations.
- MIT license, contribution guide, provenance statement, and explicit security scope.
- Evidence ledger, ADRs, requirements, roadmap, benchmark plan, and validation record.
- Git ignore rules exclude build output, proprietary binary formats, common credential files, and private research directories.
- CI workflow checks stable and the declared minimum Rust version on Windows, Linux, and macOS. Checkout is pinned to a resolved commit and credentials are not persisted. Hosted checks passed on all six OS/toolchain combinations after the supervised-core upgrade; see [validation](VALIDATION.md).

## Phase 1 publication checks

1. Owner/repository: `topherchris420/james`. Description identifies Phase 1 research, architecture, and the authorization core. The owner maintains the security channel; private vulnerability reporting was enabled and verified on 2026-10-11.
2. The initial file list and source provenance were reviewed. The core has no third-party dependencies. Research artifacts and raw captures remain outside the repository; a common credential-pattern scan found no matches.
3. README explicitly identifies the text-chat preview status. A GitHub Actions Windows executable and localhost model support exist; signed installers, research integration and sandbox guarantees remain future work.
4. Initial local formatting, 26 contract tests, strict Clippy, and the demonstration passed before the first push. The update adds seven supervisor contract tests and a second simulated demonstration. Hosted CI has passed on Windows, Linux and macOS with stable and Rust 1.85.0; this is compile/test evidence, not a desktop-runtime release gate. Pull request #1 was reviewed through CI and merged to `main`; repository-wide branch protection is a separate administration decision.
5. Public findings retain source links and incomplete-analysis qualifications. The proprietary reference bundle is not included.

## Before a usable desktop release

Complete the roadmap's end-to-end, recovery, privacy, accessibility, packaging/update, and platform gates. Generate a dependency/license inventory and a signed release manifest containing artifact hashes. Configure release ownership and supported-version policy. Publish benchmark methodology and raw synthetic-task results before asserting any comparative advantage.

The repository license does not authorize redistribution of reference artifacts. James's scope and claims should reflect its actual released implementation.


## Desktop preview distribution

The Windows desktop job builds `target/release/james.exe` and uploads `James-Windows-x64` using GitHub Actions. Downloading the artifact requires extracting the archive and separately setting up LM Studio with model weights. It is unsigned, is not an MSI installer, and is not tested against all Windows configurations. No third-party reference installer is redistributed. See [DESKTOP.md](DESKTOP.md).
