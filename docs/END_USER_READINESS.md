# End-user readiness

This is the release gate for James as software that ordinary people install
and use. It scores evidence, not effort: a criterion earns credit only when
the evidence named beside it exists in [VALIDATION.md](VALIDATION.md) or in
CI. Implementation that has not been checked where it matters earns nothing.

**Assessment for James 0.4.0 (2026-10-12): 5.2 / 10 — Developer preview.**

James 0.4.0 is not ready to be called a public beta, release candidate or
production-ready release, and this page explains exactly why and what would
change that.

## How the score works

Each category has criteria. A criterion scores:

- **1** — verified with the evidence it asks for, on every platform it names;
- **½** — verified only in part: some platforms, a container rather than a
  real desktop, automated tests where a real run is required, or a small
  sample;
- **0** — not verified, blocked, or failed.

A category's score is its criteria's average, out of 10. The total is the
weighted sum. Automated tests earn full credit only for criteria that are
about code paths (protocol handling, fault injection, parsing); criteria
about people, devices or real software require real runs.

## Release classifications

| Classification | Requires | James 0.4.0 |
| --- | --- | --- |
| **Developer preview** | Development and CI evidence; limitations stated prominently | **Yes** |
| **Public beta** | Installation and the primary workflows (install, connect, chat, stop, restart, update, uninstall) verified on real hardware for each platform the release names, with limitations disclosed | No: Windows and macOS have no real-device run; Linux only in a container |
| **Release candidate** | Every category's required criteria pass, all 18 [acceptance scenarios](ACCEPTANCE_TESTS.md) pass on every named platform, no open release-blocking defect | No |
| **Production-ready** | Release candidate, plus signed releases, an independent security review, accessibility testing with screen readers, and an end-user study meeting the published targets | No |

A score never upgrades a classification. Each classification also requires
every hard gate below for the platforms it names.

## Hard gates

A 10/10, and any classification above developer preview, is blocked while
any of these is open:

| Gate | Status |
| --- | --- |
| Real-machine install, first launch, setup to a first answer, upgrade and uninstall on Windows, macOS and Linux | **Open.** Linux done in a container (real packages, real models, virtual display); Windows and macOS never run by a person |
| Recovery from a crash or power loss during an approved action, on Windows hardware | **Open.** Automated, and a 0.3 record recovered after an upgrade on Linux; never with a real Windows crash |
| Action safety with real input on Windows: deny, expiry, target-window refusal, countdown cancel, one input, reconciliation | **Open.** Deny and Cancel are GUI-tested on the Windows CI runner and the helper's refusal is checked there; **no real input has been sent to an application in 0.4.0 testing** |
| Privacy: what is stored, where, with what permissions, and deletion, checked on each platform | **Open.** Verified on Linux; Windows ACLs and macOS not checked |
| Accessibility with NVDA or Narrator, VoiceOver and Orca, keyboard-only use, 200% text | **Open.** Keyboard and scaling are tested; no screen reader has been used |
| Signed Windows installer and notarized macOS app | **Open.** No certificates exist; the pipeline supports them when configured |

## Scorecard

### 1. Installation and onboarding — 20% — 3.1 / 10

| # | Criterion | Evidence | Score |
| --- | --- | --- | --- |
| 1.1 | Packages for Windows, macOS and Linux build reproducibly from a commit, with checksums | CI builds the installer, portable `.exe`, universal app, `.deb` and archive on every push; release job verifies checksums, adds provenance and metadata | 1 |
| 1.2 | Clean install, launch, upgrade and uninstall on a real Linux desktop | Ubuntu 24.04 container: apt install, 0.3.0 → 0.4.0 upgrade keeping data, apt remove; archive install/uninstall. Not a physical desktop | ½ |
| 1.3 | The same on real Windows 10/11 | Not run | 0 |
| 1.4 | The same on real macOS (Apple silicon and Intel) | Not run | 0 |
| 1.5 | Guided setup reaches a real model's answer without a terminal | Done in the container with real Ollama; but pulling an Ollama model on Linux needs a terminal, and the LM Studio path (the terminal-free one) was not run | ½ |
| 1.6 | Missing, stopped and empty servers are explained with a next step | Real Ollama and llama.cpp for found / missing model / not running; the "running without models" case only against scripted servers | ½ |
| 1.7 | Builds trusted by the OS (signed, notarized) | Unsigned | 0 |
| 1.8 | A nontechnical person installs and gets a first answer unaided | Not run | 0 |

### 2. Reliability and recovery — 20% — 6.3 / 10

| # | Criterion | Evidence | Score |
| --- | --- | --- | --- |
| 2.1 | Streaming and Stop against real LM Studio, Ollama and llama.cpp | Ollama and llama.cpp real; Ollama confirmed to cancel generation on Stop; LM Studio not run | ½ |
| 2.2 | Fault handling: disconnects, malformed and oversized replies, idle and total timeouts | Socket-fixture tests in `james-loopback` and `james-models`, GUI tests for Stop and failures | 1 |
| 2.3 | Plain-language errors for missing model, full context, no image support, non-chat model, sign-in | Each triggered on real Ollama or llama.cpp and classified correctly | 1 |
| 2.4 | History and settings survive restart and update | Linux package upgrade with real data; other platforms automated only | ½ |
| 2.5 | A crash during an unresolved action is explained and never replayed | Automated; old-format record recovered on Linux; no real Windows crash | ½ |
| 2.6 | Denial, expiry and cancellation send nothing | Automated (timing, worker, GUI tests on the Windows runner); no real Windows input | ½ |
| 2.7 | The R.A.I.N. Lab being unavailable never breaks James | Real Lab connected; offline and back-off automated | 1 |
| 2.8 | Long sessions, sleep/resume and low-memory behaviour | Not run | 0 |

### 3. Security and privacy — 20% — 5.6 / 10

| # | Criterion | Evidence | Score |
| --- | --- | --- | --- |
| 3.1 | Network limited to loopback; no hidden cloud fallback | `james-loopback` design and tests; no other HTTP client in the workspace | 1 |
| 3.2 | One-use, exact, expiring, deny-by-default approval with intent recorded before input | Contract, review and GUI tests; real Windows input not exercised | ½ |
| 3.3 | Input only reaches the reviewed window | Windows helper checks the window in front and under a click; refusal path tested on the Windows CI runner; never with a real target app | ½ |
| 3.4 | Model output cannot grant or chain actions | Adversarial proposal tests (duplicates, look-alikes, chords, lists, self-approval, numbers); voice cannot approve | 1 |
| 3.5 | Stored data private, retention controllable, deletion works | Owner-only files and controls verified on Linux; not encrypted; Windows and macOS not checked | ½ |
| 3.6 | Remote models disclosed, never chosen silently | Real Ollama cloud model labelled and warned | 1 |
| 3.7 | Independent security review | Not done | 0 |
| 3.8 | Signed releases with verifiable provenance | Attestation and metadata configured; never run (no tag); unsigned | 0 |

### 4. User experience and accessibility — 15% — 5.0 / 10

| # | Criterion | Evidence | Score |
| --- | --- | --- | --- |
| 4.1 | Consistent, polished interface; readable Markdown; useful empty states | Real sessions and screenshots; two rendering bugs found and fixed | 1 |
| 4.2 | Keyboard-only operation with visible focus and discoverable shortcuts | GUI tests; Tab and shortcuts on X11 | 1 |
| 4.3 | Screen readers work | AccessKit enabled; no screen reader used | 0 |
| 4.4 | High contrast, 200% text and reduced motion usable | Contrast test (WCAG AA/AAA), layout tests; no human check | ½ |
| 4.5 | High-DPI, small windows and multiple monitors | Layout tests at 1x/1.25x/2x and 760×560; real 2x session; multiple monitors not tested | ½ |
| 4.6 | Usability study with first-time users | Not run | 0 |

### 5. Cross-platform quality — 10% — 5.0 / 10

| # | Criterion | Evidence | Score |
| --- | --- | --- | --- |
| 5.1 | Builds, tests and lints on all three in CI | Windows, macOS, Ubuntu with stable; library crates with Rust 1.85, desktop with 1.88 | 1 |
| 5.2 | Real Linux desktop (X11 and Wayland, GPU) | Xvfb container only | ½ |
| 5.3 | Real Windows device | Not run | 0 |
| 5.4 | Real Mac, Apple silicon and Intel | Not run | 0 |
| 5.5 | Unsupported features are disabled and explained, never faked | Platform matrix; GUI test for macOS/Linux input | 1 |

### 6. Model compatibility — 10% — 6.0 / 10

| # | Criterion | Evidence | Score |
| --- | --- | --- | --- |
| 6.1 | Ollama | 0.40.3: chat, streaming, Stop, vision, capabilities, cloud flag, errors | 1 |
| 6.2 | llama.cpp | b11564: chat, streaming, vision, capabilities, errors | 1 |
| 6.3 | LM Studio | Fixtures only | 0 |
| 6.4 | Other OpenAI-compatible servers | llama-cpp-python in an earlier session; fixtures | ½ |
| 6.5 | Vision answers useful with recommended models | Gemma 3 4B correct on two samples; SmolVLM 500M not; tiny sample | ½ |

### 7. Documentation and support — 5% — 7.5 / 10

| # | Criterion | Evidence | Score |
| --- | --- | --- | --- |
| 7.1 | README and guide accurate, with authentic screenshots | Rewritten for 0.4.0 from this session's real screenshots | 1 |
| 7.2 | Troubleshooting covers the failures people actually hit | Every error class observed on real servers has an entry | 1 |
| 7.3 | Limitations, platform matrix and evidence published | This page, [PLATFORM_SUPPORT.md](PLATFORM_SUPPORT.md), [VALIDATION.md](VALIDATION.md) | 1 |
| 7.4 | Guide tested with a nontechnical reader | Not run | 0 |

### Total

| Category | Weight | Score | Weighted |
| --- | --- | --- | --- |
| Installation and onboarding | 20% | 3.1 | 0.63 |
| Reliability and recovery | 20% | 6.3 | 1.25 |
| Security and privacy | 20% | 5.6 | 1.13 |
| User experience and accessibility | 15% | 5.0 | 0.75 |
| Cross-platform quality | 10% | 5.0 | 0.50 |
| Model compatibility | 10% | 6.0 | 0.60 |
| Documentation and support | 5% | 7.5 | 0.38 |
| **Total** | | | **5.2 / 10** |

## What would raise the score

Ordered by how much each would move the assessment. Each links to the exact
procedure.

1. **Windows and macOS on real machines** (1.3, 1.4, 2.4-2.6, 3.2, 3.3, 3.5, 5.3, 5.4; opens the path to public beta): run [acceptance scenarios](ACCEPTANCE_TESTS.md) 1-16 on Windows 11 x64 and on Apple silicon and Intel Macs, including one real reviewed key press in Notepad or TextEdit and a forced quit during the countdown.
2. **LM Studio** (1.5, 1.6, 2.1, 6.3): scenarios 3-6 and 12 with LM Studio, including a model that is listed but not loaded.
3. **Screen readers** (4.3): scenario 18 with NVDA, VoiceOver and Orca; file each problem.
4. **People** (1.8, 4.6, 7.4): scenario 17 with at least five people who have never used a local model, on their own computers.
5. **Signing** (1.7, 3.8): add an Authenticode certificate and a Developer ID as repository secrets (see [INSTALLER.md](INSTALLER.md#signing)), push a tag, and verify the release's attestations and `release-metadata.json`.
6. **Security review** (3.7): an independent review of `james-loopback`, the review and input path, the intent record and the Windows helpers.
7. **Soak and power events** (2.8): a two-hour session with sleep/resume and a model swap in the middle.

## Known release blockers and limitations

- No physical Windows or Mac verification of anything in 0.4.0.
- No real input has been sent by 0.4.0; the Windows target-window check has only been exercised on its refusal path.
- Builds are unsigned: Windows SmartScreen and macOS Gatekeeper warn on first launch.
- Not encrypted at rest; protection relies on the OS account and full-disk encryption.
- Screen reader support is implemented through AccessKit but untested.
- Talk and Dictate were not exercised (no microphone in the test environment).
- Small local models can give poor or irrelevant answers; James cannot fix that, only report what the model says.
