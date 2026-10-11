# James benchmark and validation protocol

Status: **proposed protocol**, 2026-10-11. No comparative James/Korso benchmark results are established here. Phase 1 core tests validate only their exercised policy/lifecycle contracts; they do not demonstrate desktop reliability, sandboxing, local inference, or product superiority.

## Questions to answer

| Claim area | Measurement | Required comparison |
| --- | --- | --- |
| Extensibility | Correct discovery/execution, installation effort, incompatibilities, failures | Matched integration and permissions |
| Local-model support | Verified model/provider combinations and offline task completion | Same task and demonstrable inference locality |
| Coordination | Accepted task completion, elapsed time, tokens, conflicting edits | James single-agent, James coordinated, reference where supported |
| Reliability | Recovery outcomes, duplicate side effects, cancellation latency | Same injected faults and observable result |
| Transparency | Accurate visible destination/action/state; accessible review | Defined inspection tasks and evidence checks |
| User control | Denials, revocations, scope enforcement, unauthorized side effects | Same requested authority and test resources |

Feature inventory, marketing statements, static artifact findings, and a successful demo are evidence of different kinds. None substitutes for measured comparative outcomes. Report reference features that cannot be accessed or matched as **not measured** rather than scoring their presumed absence.

## Phase 1 contract verification

Use deterministic tests against the actual core prototype. The following are acceptance targets, not a claim that all tests already exist or pass.

| Trigger | Expected observable result |
| --- | --- |
| Invocation arguments change after review | Original grant cannot authorize the changed invocation |
| Run, action identity, or action revision differs | Authorization denied |
| Budget is exhausted | Further dispatch denied without charging; future supervisor records the run's halt |
| Illegal task-state transition | Transition rejected without corrupting the prior state |
| Process restarts with saved approval/task state | Transient authority invalidated; uncertain work cannot silently resume |
| Action is denied or cancelled before dispatch | No dispatch authorization issued |

Tests should assert consequences at the dispatch contract, not merely compare getters or mirror implementation branches. Later integration tests must prove that adapters cannot bypass this contract. Keep actual test names, commands, revisions, and output in validation records.

## Comparison configuration

Freeze the task corpus, scoring rubric, fixtures, success criteria, and analysis before comparative runs. Record:

- James commit, build profile, dependencies, executable hash, and enabled features.
- Korso executable/version/hash where available, investigation evidence identifiers, and observable configuration.
- OS/build, CPU, RAM, GPU/driver, VRAM, storage, and power profile.
- Provider/server version, endpoint, inference locality, model identifier/digest, quantization, context length, sampling settings, and tool-call template where known.
- Tool/extension versions, fixture hashes, permissions, network conditions, concurrency, and task budgets.
- Cold/warm status, model loading state, caches, request/response sizes, and trial order.

Create two distinct analyses: **controlled comparisons** with the same model/backend where feasible, and **product configuration comparisons** using documented configurations. Do not attribute model differences to James's architecture. If parity is impossible, describe the mismatch and narrow the claim.

Use paired trials on the same fixture and randomize app order to reduce ordering effects. Include warm-up trials separately from scored runs. Report cold start/model-load timing separately from warm inference. Repeat enough trials to characterize variation; document the selected trial count and statistical method before collecting the results. Preserve failed and timed-out trials.

## Task corpus

Start with bounded synthetic workspaces and disposable external fixtures:

1. Conversation and summarization with independently checkable required facts.
2. File search and a small authorized edit with expected output/diff.
3. A tool sequence with a dependency and an observable final artifact.
4. A multi-worker research/edit task with possible conflicting proposals.
5. Provider discovery, unavailable model, structured output, and unsupported tool-call handling.
6. Permission denial, argument revision, revocation, and attempted out-of-scope access.
7. Restart and recovery around a simulated side effect.
8. Offline operation with a verified on-device backend and measured network behavior.

Keep tasks representative of the requested capabilities, and document excluded workflows. Score artifacts using automated checks where practical and blinded rubric review where judgment is necessary. An agent's self-reported success does not establish task completion.

## Metrics and accounting

Report task acceptance rate with denominators, partial completion separately, and the reason for every failure. Measure wall-clock task duration, time to first token, full-response latency, model-load time, prompt/output tokens, monetary cost if applicable, tool count, retries, RAM/VRAM, CPU/GPU use, and UI input latency under load.

For coordinated runs, include every worker's requests, tokens, retries, elapsed time, and abandoned work. Compare against James's single-agent baseline before claiming coordination improves outcomes. Token usage reported by a provider may be missing or incompatible; record its origin and leave unavailable fields unknown.

Measure cancellation from user intent to dispatch prevention, UI acknowledgement, and confirmed backend/process termination separately. A closed stream is not proof that inference or an external tool stopped.

Record unauthorized reads, writes, network disclosures, and executions against controlled probes. Passing probes demonstrate coverage of those cases, not universal isolation.

## Fault and recovery matrix

Inject faults at documented boundaries:

| Fault | Required observation |
| --- | --- |
| Provider unreachable, unavailable model, timeout, rate limit | Clear failure; bounded retry; no silent provider change |
| Malformed/truncated/oversized stream | Bounded resource use; incomplete output; accurate final status |
| Tool hangs, crashes, writes invalid protocol, spawns child | Deadline and process-tree handling; preserved diagnostics |
| Approval revoked or arguments changed before dispatch | Side effect prevented |
| Termination before dispatch | Safe recovery without unintended execution |
| Termination after side effect, before durable result | Unknown outcome shown; no blind repeat |
| Concurrent edits to one artifact | Conflict detected or serialized; no silent lost update |
| Database locked, full disk, interrupted migration | Explicit failure; retained/recoverable prior data |
| Symlink/reparse/traversal change near authorization | Boundary violation prevented for the tested race |
| Budget exhaustion during worker fan-out | No unbounded new work; consistent accounting |

Use a controllable fake provider/tool to hit fault boundaries deterministically, then repeat meaningful cases with actual supported providers and OS processes. Do not substitute mocks for the complete integration path.

## Locality and extension coverage

Verify inference locality separately from the hostname: a local server can relay to a cloud model. Document provider configuration and test offline/network behavior. Distinguish enforcement over James's connections from enforcement over the external inference process. [Ollama API locality distinctions](https://docs.ollama.com/api/introduction).

For MCP, test supported protocol versions and valid/invalid schemas, duplicate tool names, missing capabilities, timeouts, output limits, credentials, and destination validation. Current protocol behavior differs from older profiles. [MCP version changes](https://modelcontextprotocol.io/specification/2026-07-28/changelog), [Official Rust SDK](https://github.com/modelcontextprotocol/rust-sdk).

Subprocess supervision and an MCP connection do not establish OS isolation. Any sandbox claim needs independent probes for filesystem, network, environment, child processes, and escape handling on each supported platform. [MCP security principles](https://modelcontextprotocol.io/specification/2026-07-28), [Windows process management](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects).

## Artifacts and claim gate

Each benchmark run should produce machine-readable configuration and trial records, raw timing/events, artifact diffs, failure evidence, and a human-readable report. Store sensitive fixtures outside publishable bundles; redact credentials and personal data while preserving the information needed to reproduce synthetic trials.

Reports must distinguish **observed**, **inferred**, **unknown**, and **proposed**. Include hardware, version/model parity limits, trial counts, spread/confidence intervals where appropriate, exclusions, and all failures. Preserve REA evidence references for reference-derived observations without redistributing proprietary implementation material.

A superiority claim is permitted only for a named metric and tested configuration when matched comparative results support it. For example, report a measured recovery success difference with trial counts and uncertainty; do not generalize it to overall reliability. Until then, use design goals and measured James-only results without ranking James above Korso.
