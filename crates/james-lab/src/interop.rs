//! `agent-interop/v1` client for a R.A.I.N. Lab that its operator has
//! configured for interop (`docs/interop/agent-interop-v1/README.md`).
//!
//! Least privilege on James's side:
//! - only three requests exist: read status, read one episode's evidence,
//!   submit one proposal for review;
//! - tokens are held by the caller in memory and sent only as a bearer header
//!   over `james-loopback`, which cannot leave this computer;
//! - evidence is refused unless its SHA-256 recomputes under the shared
//!   canonical rule (and matches the digest the status listed, when given);
//! - a proposal is sent only with the person's approval of its exact digest,
//!   and a receipt that claims to authorize execution is refused.
//!
//! Lab text is data. It is cleaned, bounded and labelled, never executed and
//! never treated as an instruction. A digest is not a signature: a process
//! that controls the Lab port can serve any self-consistent record.

use crate::LabError;
use james_interop::canonical;
use james_loopback::{Client, Error as HttpError, Limits, Method, Timeouts};
use serde_json::{Value, json};
use std::fmt;
use std::time::Duration;

const MAX_EVIDENCE: usize = 256 * 1024;
const MAX_REQUEST: usize = 16 * 1024;
const TIMEOUT: Duration = Duration::from_secs(5);

pub const STATUS_PATH: &str = "/api/rain/interop/v1/status";
pub const EVIDENCE_PATH: &str = "/api/rain/interop/v1/evidence";
pub const PROPOSALS_PATH: &str = "/api/rain/interop/v1/proposals";

/// A bearer token James keeps in memory only.
#[derive(Clone, PartialEq, Eq)]
pub struct Token(String);

impl Token {
    pub fn new(text: &str) -> Option<Self> {
        let t = text.trim();
        (8..=512)
            .contains(&t.len())
            .then(|| t.chars().all(|c| c.is_ascii_graphic()))
            .filter(|ok| *ok)
            .map(|_| Self(t.to_owned()))
    }
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Token(…)")
    }
}

#[derive(Debug, Clone)]
pub struct InteropClient {
    port: u16,
    read: Token,
    propose: Option<Token>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpisodeSummary {
    pub episode_id: String,
    pub title: String,
    pub outcome: String,
    pub evidence_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteropStatus {
    pub lab_commit: Option<String>,
    pub dirty: Option<bool>,
    pub scopes: Vec<String>,
    pub episodes: Vec<EpisodeSummary>,
    pub proposals_pending: u64,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Claim {
    pub claim_id: String,
    pub statement: String,
    pub author: String,
    pub generation: String,
    pub standing: String,
    pub evidence_run_ids: Vec<String>,
    pub open_challenges: u64,
    pub supersedes: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Run {
    pub run_id: String,
    pub purpose: String,
    pub record_sha256: String,
    pub status: String,
    pub verdict: String,
    pub measurements: Vec<(String, Option<f64>)>,
    pub replay_verified: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Challenge {
    pub challenge_id: String,
    pub by: String,
    pub target_claim_id: String,
    pub text: String,
    pub generation: String,
    pub resolution: String,
    pub resolved_by_run: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Evidence {
    pub episode_id: String,
    pub title: String,
    pub question: String,
    pub outcome: String,
    pub claims: Vec<Claim>,
    pub runs: Vec<Run>,
    pub challenges: Vec<Challenge>,
    pub limitations: Vec<String>,
    pub lab_commit: Option<String>,
    pub protocol_sha256: String,
    pub journal_head_sha256: String,
    pub roster_sha256: String,
    pub sha256: String,
}

/// What the person approved, bound to the proposal's digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonApproval {
    pub request_id: String,
    pub action_sha256: String,
    pub decided_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalDraft {
    pub agent_id: String,
    pub title: String,
    pub question: String,
    pub hypothesis: String,
    pub rationale: String,
    /// `person`, `model` or `scripted`: who wrote the words.
    pub generation: String,
}

impl ProposalDraft {
    /// The exact action a person approves: everything but the approval.
    pub fn action(&self) -> Value {
        json!({
            "kind": "proposal_submission",
            "from": { "system": "james-desktop", "agent_id": self.agent_id },
            "title": self.title,
            "question": self.question,
            "hypothesis": self.hypothesis,
            "rationale": self.rationale,
            "generation": self.generation,
        })
    }
    pub fn action_sha256(&self) -> String {
        canonical::digest(&self.action()).expect("text fields are finite")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    pub proposal_sha256: String,
    pub status: String,
    pub queue_position: u64,
    pub note: String,
}

fn clean(text: &str, max: usize) -> String {
    let mut out: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(max)
        .collect();
    if text.chars().count() > max {
        out.push('…');
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn bad(what: &str) -> LabError {
    LabError(format!(
        "The Lab's interop answer is not valid agent-interop/v1: {what}"
    ))
}

fn text(v: &Value, key: &str, max: usize) -> Result<String, LabError> {
    v.get(key)
        .and_then(Value::as_str)
        .filter(|s| s.chars().count() <= max)
        .map(|s| clean(s, max))
        .ok_or_else(|| bad(key))
}

fn opt_text(v: &Value, key: &str, max: usize) -> Result<Option<String>, LabError> {
    match v.get(key) {
        Some(Value::Null) => Ok(None),
        Some(Value::String(s)) if s.chars().count() <= max => Ok(Some(clean(s, max))),
        _ => Err(bad(key)),
    }
}

fn hex64(v: &Value, key: &str) -> Result<String, LabError> {
    let s = v.get(key).and_then(Value::as_str).ok_or_else(|| bad(key))?;
    (s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))
    .then(|| s.to_owned())
    .ok_or_else(|| bad(key))
}

fn one_of(v: &Value, key: &str, allowed: &[&str]) -> Result<String, LabError> {
    v.get(key)
        .and_then(Value::as_str)
        .filter(|s| allowed.contains(s))
        .map(str::to_owned)
        .ok_or_else(|| bad(key))
}

fn list<'a>(v: &'a Value, key: &str, max: usize) -> Result<&'a Vec<Value>, LabError> {
    v.get(key)
        .and_then(Value::as_array)
        .filter(|a| a.len() <= max)
        .ok_or_else(|| bad(key))
}

pub fn valid_episode_id(id: &str) -> bool {
    let b = id.as_bytes();
    !b.is_empty()
        && b.len() <= 96
        && b[0].is_ascii_alphanumeric()
        && b.iter()
            .all(|c| c.is_ascii_alphanumeric() || b"_.:-".contains(c))
}

impl InteropClient {
    pub fn new(port: u16, read: Token, propose: Option<Token>) -> Result<Self, LabError> {
        if port == 0 {
            return Err(LabError("Choose a nonzero R.A.I.N. Lab port".into()));
        }
        Ok(Self {
            port,
            read,
            propose,
        })
    }

    fn exchange(
        &self,
        method: Method,
        path: &str,
        token: &Token,
        body: Option<&[u8]>,
        cap: usize,
    ) -> Result<Value, LabError> {
        let client = Client::new(self.port)
            .map_err(|e| LabError(e.to_string()))?
            .with_timeouts(Timeouts {
                connect: TIMEOUT,
                idle: TIMEOUT,
                total: Some(TIMEOUT),
            })
            .with_limits(Limits {
                headers: 16 * 1024,
                body: cap,
            });
        let mut request = client
            .request(method, path)
            .header("Accept", "application/json")
            .header("Authorization", format!("Bearer {}", token.0));
        if let Some(bytes) = body {
            request = request.body("application/json", bytes);
        }
        let response = request.send().map_err(|e| {
            LabError(match e {
                HttpError::Unreachable(_) | HttpError::TimedOut | HttpError::Io(_) => {
                    "Cannot reach the R.A.I.N. Lab on that local port within 5 seconds".into()
                }
                other => format!("The R.A.I.N. Lab returned an unexpected HTTP response: {other}"),
            })
        })?;
        let status = response.status();
        if response.media_type().as_deref() != Some("application/json") {
            return Err(LabError(format!(
                "The Lab answered HTTP {status} without JSON; interop may be off on that port"
            )));
        }
        let bytes = response.read_to_end(cap).map_err(|e| {
            LabError(match e {
                HttpError::TooLarge => {
                    "The Lab's interop answer is larger than James accepts".into()
                }
                other => format!("Reading the Lab's answer failed: {other}"),
            })
        })?;
        let value: Value = serde_json::from_slice(&bytes).map_err(|_| bad("not JSON"))?;
        if status != 200 {
            let code = value
                .get("code")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let message = value
                .get("message")
                .and_then(Value::as_str)
                .map(|m| clean(m, 300))
                .unwrap_or_default();
            return Err(LabError(match (status, code) {
                (401, _) => {
                    "The Lab refused the token (HTTP 401). Check the token its operator gave you."
                        .into()
                }
                (403, "forbidden_scope") => {
                    "That token does not allow this request (HTTP 403).".into()
                }
                (404, "not_configured") => {
                    "Interop is off in that Lab. Its operator turns it on with a token.".into()
                }
                _ => format!(
                    "The Lab refused the request (HTTP {status}, {}): {message}",
                    clean(code, 40)
                ),
            }));
        }
        Ok(value)
    }

    pub fn status(&self) -> Result<InteropStatus, LabError> {
        let v = self.exchange(Method::Get, STATUS_PATH, &self.read, None, MAX_EVIDENCE)?;
        if v.get("schema").and_then(Value::as_str) != Some("rain-interop-status/v1")
            || v.get("interop_version").and_then(Value::as_str)
                != Some(james_interop::INTEROP_VERSION)
        {
            return Err(bad("schema or version"));
        }
        let lab = v.get("lab").ok_or_else(|| bad("lab"))?;
        let episodes = list(&v, "episodes", 64)?
            .iter()
            .map(|e| {
                let id = text(e, "episode_id", 96)?;
                if !valid_episode_id(&id) {
                    return Err(bad("episode_id"));
                }
                Ok(EpisodeSummary {
                    episode_id: id,
                    title: text(e, "title", 160)?,
                    outcome: one_of(
                        e,
                        "outcome",
                        &["supported", "refuted", "inconclusive", "contested"],
                    )?,
                    evidence_sha256: hex64(e, "evidence_sha256")?,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(InteropStatus {
            lab_commit: opt_text(lab, "commit", 40)?,
            dirty: lab.get("dirty").and_then(Value::as_bool),
            scopes: list(&v, "scopes_enabled", 2)?
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect(),
            episodes,
            proposals_pending: v
                .get("proposals_pending")
                .and_then(Value::as_u64)
                .ok_or_else(|| bad("proposals_pending"))?,
            limitations: list(&v, "limitations", 8)?
                .iter()
                .filter_map(Value::as_str)
                .map(|s| clean(s, 400))
                .collect(),
        })
    }

    /// One episode's evidence, refused unless its digest recomputes (and
    /// equals `expected_sha256` when the caller has one from the status).
    pub fn evidence(
        &self,
        episode_id: &str,
        expected_sha256: Option<&str>,
    ) -> Result<Evidence, LabError> {
        if !valid_episode_id(episode_id) {
            return Err(LabError("That is not an episode id".into()));
        }
        let path = format!("{EVIDENCE_PATH}?episode={episode_id}");
        let v = self.exchange(Method::Get, &path, &self.read, None, MAX_EVIDENCE)?;
        parse_evidence(&v, expected_sha256)
    }

    /// Sends one proposal the person approved. The Lab only queues it.
    pub fn submit(
        &self,
        draft: &ProposalDraft,
        approval: &PersonApproval,
    ) -> Result<Receipt, LabError> {
        let Some(token) = &self.propose else {
            return Err(LabError(
                "No proposal token is set; the Lab operator decides whether James may propose"
                    .into(),
            ));
        };
        if approval.action_sha256 != draft.action_sha256() {
            return Err(LabError(
                "What was approved is not this proposal; nothing was sent".into(),
            ));
        }
        let proposal = canonical::seal(json!({
            "schema": "rain-interop-proposal/v1",
            "from": { "system": "james-desktop", "agent_id": draft.agent_id },
            "title": draft.title,
            "question": draft.question,
            "hypothesis": draft.hypothesis,
            "rationale": draft.rationale,
            "generation": draft.generation,
            "approval": {
                "request_id": approval.request_id,
                "action_sha256": approval.action_sha256,
                "decided_at": approval.decided_at,
            },
        }))
        .map_err(|e| LabError(e.to_string()))?;
        let body = canonical::canonical(&proposal).map_err(|e| LabError(e.to_string()))?;
        if body.len() > MAX_REQUEST {
            return Err(LabError(
                "The proposal is larger than the Lab accepts (16 KiB)".into(),
            ));
        }
        let v = self.exchange(
            Method::Post,
            PROPOSALS_PATH,
            token,
            Some(body.as_bytes()),
            MAX_REQUEST,
        )?;
        if v.get("schema").and_then(Value::as_str) != Some("rain-interop-receipt/v1") {
            return Err(bad("receipt schema"));
        }
        if v.get("authorizes_execution") != Some(&Value::Bool(false)) {
            return Err(LabError(
                "The Lab's receipt claims the proposal authorizes execution; James refuses it"
                    .into(),
            ));
        }
        let receipt = Receipt {
            proposal_sha256: hex64(&v, "proposal_sha256")?,
            status: one_of(&v, "status", &["queued_for_operator_review", "duplicate"])?,
            queue_position: v
                .get("queue_position")
                .and_then(Value::as_u64)
                .ok_or_else(|| bad("queue_position"))?,
            note: text(&v, "note", 400)?,
        };
        if receipt.proposal_sha256 != proposal["sha256"].as_str().unwrap_or_default() {
            return Err(bad("the receipt names a different proposal"));
        }
        Ok(receipt)
    }
}

pub fn parse_evidence(v: &Value, expected_sha256: Option<&str>) -> Result<Evidence, LabError> {
    if v.get("schema").and_then(Value::as_str) != Some("rain-interop-evidence/v1") {
        return Err(bad("schema"));
    }
    if !canonical::verify_sealed(v) {
        return Err(LabError(
            "The evidence's SHA-256 does not match its contents; James refuses it".into(),
        ));
    }
    let sha256 = hex64(v, "sha256")?;
    if expected_sha256.is_some_and(|e| e != sha256) {
        return Err(LabError(
            "The evidence is not the record the Lab's status listed; James refuses it".into(),
        ));
    }
    let generations = ["model", "scripted", "person", "host"];
    let claims = list(v, "claims", 32)?
        .iter()
        .map(|c| {
            Ok(Claim {
                claim_id: text(c, "claim_id", 96)?,
                statement: text(c, "statement", 600)?,
                author: text(c, "author", 64)?,
                generation: one_of(c, "generation", &generations)?,
                standing: one_of(
                    c,
                    "standing",
                    &[
                        "proposed",
                        "supported",
                        "refuted",
                        "contested",
                        "insufficient",
                        "superseded",
                    ],
                )?,
                evidence_run_ids: list(c, "evidence_run_ids", 64)?
                    .iter()
                    .filter_map(Value::as_str)
                    .map(|s| clean(s, 96))
                    .collect(),
                open_challenges: c
                    .get("open_challenges")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| bad("open_challenges"))?,
                supersedes: opt_text(c, "supersedes", 96)?,
            })
        })
        .collect::<Result<Vec<_>, LabError>>()?;
    let runs = list(v, "runs", 64)?
        .iter()
        .map(|r| {
            let measurements = r
                .get("measurements")
                .and_then(Value::as_object)
                .filter(|m| m.len() <= 32)
                .ok_or_else(|| bad("measurements"))?
                .iter()
                .map(|(k, val)| match val {
                    Value::Null => Ok((clean(k, 64), None)),
                    Value::Number(n) => Ok((clean(k, 64), n.as_f64())),
                    _ => Err(bad("measurement")),
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Run {
                run_id: text(r, "run_id", 96)?,
                purpose: text(r, "purpose", 200)?,
                record_sha256: hex64(r, "record_sha256")?,
                status: one_of(r, "status", &["passed", "failed", "inconclusive", "error"])?,
                verdict: one_of(
                    r,
                    "verdict",
                    &[
                        "supported",
                        "not_supported",
                        "insufficient_evidence",
                        "not_evaluated",
                    ],
                )?,
                measurements,
                replay_verified: r
                    .get("replay_verified")
                    .and_then(Value::as_bool)
                    .ok_or_else(|| bad("replay_verified"))?,
            })
        })
        .collect::<Result<Vec<_>, LabError>>()?;
    let challenges = list(v, "challenges", 64)?
        .iter()
        .map(|c| {
            Ok(Challenge {
                challenge_id: text(c, "challenge_id", 96)?,
                by: text(c, "by", 64)?,
                target_claim_id: text(c, "target_claim_id", 96)?,
                text: text(c, "text", 600)?,
                generation: one_of(c, "generation", &generations)?,
                resolution: one_of(c, "resolution", &["open", "upheld", "rejected"])?,
                resolved_by_run: opt_text(c, "resolved_by_run", 96)?,
            })
        })
        .collect::<Result<Vec<_>, LabError>>()?;
    let p = v.get("provenance").ok_or_else(|| bad("provenance"))?;
    if p.get("evidence_class").and_then(Value::as_str) != Some("simulated") {
        return Err(bad("evidence_class"));
    }
    Ok(Evidence {
        episode_id: text(v, "episode_id", 96)?,
        title: text(v, "title", 160)?,
        question: text(v, "question", 600)?,
        outcome: one_of(
            v,
            "outcome",
            &["supported", "refuted", "inconclusive", "contested"],
        )?,
        claims,
        runs,
        challenges,
        limitations: list(v, "limitations", 16)?
            .iter()
            .filter_map(Value::as_str)
            .map(|s| clean(s, 400))
            .collect(),
        lab_commit: opt_text(p, "lab_commit", 40)?,
        protocol_sha256: hex64(p, "protocol_sha256")?,
        journal_head_sha256: hex64(p, "journal_head_sha256")?,
        roster_sha256: hex64(p, "roster_sha256")?,
        sha256,
    })
}

impl Evidence {
    /// A host-written explanation: what was claimed, what tested it, what
    /// stands, and what the evidence cannot show. No model is involved.
    pub fn explain(&self) -> String {
        let mut out = vec![
            format!(
                "“{}” ({}), simulated evidence from the R.A.I.N. Lab.",
                self.title, self.episode_id
            ),
            format!("Question: {}", self.question),
            format!(
                "Outcome under the Lab's preregistered criteria: {}.",
                self.outcome
            ),
        ];
        for c in &self.claims {
            out.push(format!(
                "- Claim {} by {} (words: {}): “{}” — now {}{}{}.",
                c.claim_id,
                c.author,
                c.generation,
                c.statement,
                c.standing,
                if c.open_challenges > 0 {
                    format!(", {} open challenge(s)", c.open_challenges)
                } else {
                    String::new()
                },
                c.supersedes
                    .as_ref()
                    .map(|s| format!(", revises {s}"))
                    .unwrap_or_default()
            ));
        }
        for r in &self.runs {
            out.push(format!(
                "- Run {} ({}): the run's own preregistered check {} / {}; {}.",
                r.run_id,
                r.purpose,
                r.status,
                r.verdict,
                if r.replay_verified {
                    "re-executed and matched by the Lab"
                } else {
                    "not confirmed by replay"
                }
            ));
        }
        for c in &self.challenges {
            out.push(format!(
                "- Challenge {} by {} on {}: {} ({}).",
                c.challenge_id, c.by, c.target_claim_id, c.text, c.resolution
            ));
        }
        out.push("Limits:".into());
        out.extend(self.limitations.iter().map(|l| format!("- {l}")));
        out.push(format!(
            "Provenance: evidence sha256 {} (recomputed by James), protocol {}, journal head {}, lab commit {}. A digest shows the record is intact, not that it is true.",
            &self.sha256[..16],
            &self.protocol_sha256[..16],
            &self.journal_head_sha256[..16],
            self.lab_commit.as_deref().unwrap_or("not reported")
        ));
        out.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Value {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/interop/agent-interop-v1/fixtures")
            .join(name);
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
    }

    #[test]
    fn accepts_the_shared_evidence_fixture_and_refuses_tampering() {
        let v = fixture("rain-interop-evidence.json");
        let evidence = parse_evidence(&v, Some(v["sha256"].as_str().unwrap())).unwrap();
        assert_eq!(evidence.outcome, "refuted");
        assert!(evidence.explain().contains("not that it is true"));
        assert!(parse_evidence(&v, Some(&"0".repeat(64))).is_err());
        let negative = fixture("negative.json");
        let tampered = &negative["cases"][0]["value"];
        assert!(
            parse_evidence(tampered, None)
                .unwrap_err()
                .0
                .contains("does not match")
        );
    }

    #[test]
    fn a_proposal_approval_binds_to_its_exact_text() {
        let draft = ProposalDraft {
            agent_id: "researcher".into(),
            title: "t".into(),
            question: "q".into(),
            hypothesis: "h".into(),
            rationale: "r".into(),
            generation: "person".into(),
        };
        let mut changed = draft.clone();
        changed.hypothesis = "h2".into();
        assert_ne!(draft.action_sha256(), changed.action_sha256());
        let client = InteropClient::new(
            9,
            Token::new("read-token-123").unwrap(),
            Some(Token::new("propose-token-1").unwrap()),
        )
        .unwrap();
        let approval = PersonApproval {
            request_id: "A-1".into(),
            action_sha256: draft.action_sha256(),
            decided_at: "2026-11-16T12:00:00Z".into(),
        };
        assert!(
            client
                .submit(&changed, &approval)
                .unwrap_err()
                .0
                .contains("not this proposal")
        );
        assert_eq!(
            format!("{:?}", Token::new("read-token-123").unwrap()),
            "Token(…)"
        );
        assert!(Token::new("short").is_none());
    }
}
