//! `agent-interop/v1`, James's side: the canonical JSON and digest rule the
//! James desktop and the R.A.I.N. Lab share (`docs/interop/agent-interop-v1/`).
//!
//! This crate holds data rules only. It opens no connection, reads no file
//! and runs nothing; `james-agents` and `james-lab` build on it.

pub mod canonical;

/// SHA-256 of `docs/interop/agent-interop-v1/MANIFEST.sha256`. The R.A.I.N.
/// Lab pins the same constant (`src/rain/interop/contract.ts`), so a change to
/// the specification in one repository fails that repository's tests until
/// it is made, and pinned, in both.
pub const AGENT_INTEROP_V1_MANIFEST_SHA256: &str =
    "08112fdb1f3ba3770cfb2975060723909f1eb47281065467297cd46c8a07c991";

pub const INTEROP_VERSION: &str = "agent-interop/v1";
