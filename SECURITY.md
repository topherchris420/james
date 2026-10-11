# Security scope

James is a Phase 1 research and contract prototype. It has no network listener, provider credential storage, extension loader, or desktop tool executor. The current crate is not an OS security sandbox and does not validate filesystem paths or execute permissions against real operating-system resources.

`Approval::after_user_review` is a trusted-host API, not proof that a human clicked a dialog. Effect classification, path canonicalization, clock integrity, unique durable action identity, run-budget ownership, tool results, and external reconciliation belong to the future host/runtime. Host code can construct a new action or budget; the crate does not constrain arbitrary Rust callers.

For future integrations, treat prompt injection and tool output as data. Recheck concrete grants at dispatch, prevent arbitrary credential/environment inheritance, and verify the whole UI-to-executor path. A model's statement of success is not an observed outcome. A killed process is not proof that an external side effect was cancelled.

No production version is supported. The repository owner, [topherchris420](https://github.com/topherchris420), maintains the security reporting channel. GitHub private vulnerability reporting is enabled: use [Report a vulnerability](https://github.com/topherchris420/james/security/advisories/new) to share a private reproduction. Do not post credentials or private context in public issues.
