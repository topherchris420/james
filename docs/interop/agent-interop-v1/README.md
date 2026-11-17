# agent-interop/v1

A small, versioned data contract shared by the James desktop
(`topherchris420/james`) and the R.A.I.N. Lab (`topherchris420/lop-nur-twin`).
It fixes the **shape and meaning** of the records each system keeps about its
agents and, when an operator turns it on, the few messages the two systems may
exchange. It fixes nothing about how either system runs, stores, schedules or
authorises work.

This directory is vendored **byte for byte** into both repositories at
`docs/interop/agent-interop-v1/`. `MANIFEST.sha256` lists the SHA-256 of every
other file here; both test suites recompute it and compare the manifest's own
digest with a constant they pin (`AGENT_INTEROP_V1_MANIFEST_SHA256`). A change
in one copy fails that repository's tests until the same change is made, and
pinned, in the other.

## 1. Canonical JSON and digests

Every record that carries a `sha256` field is identified by
`SHA-256(canonical(record without "sha256"))`, hex, lowercase.

`canonical` is the serialisation the Lab's `canonicalJson` produces
(`src/rain/sha256.ts`), which is `JSON.stringify` of the value with object
keys sorted:

- object keys sorted by **UTF-16 code units** (JavaScript's default sort);
- no insignificant whitespace;
- strings escaped as `JSON.stringify` escapes them: `\"`, `\\`, `\b`, `\f`,
  `\n`, `\r`, `\t`, other code points below U+0020 as lowercase `\u00xx`;
  everything else, including U+2028, U+2029 and non-ASCII, is literal UTF-8;
- numbers as ECMAScript `Number::toString`: integers without a fraction
  (`1`, not `1.0`), `-0` as `0`, the shortest round-trip digits, exponent form
  only outside `[1e-7, 1e21)` (`1e+21`, `1e-7`);
- `NaN` and infinities are refused; `null`, `true`, `false` literal.

`fixtures/canonical.json` lists inputs with their exact canonical text and
digest. An implementation conforms when it reproduces every one.

A digest detects accidental or partial change. **It is not a signature**:
anyone who can rewrite a record can rewrite its digest.

## 2. Records each system keeps (its own, never the other's)

| Schema | What it records | Rules that matter |
| --- | --- | --- |
| `agent-identity/v1` | One persistent agent: id, display name, role, kind (`model-seat`, `host-seat`, `person`), closed capability list, revision | A capability is a name the host checks; an agent can never add one to itself. A revision bump is a new identity record, never an edit |
| `agent-task/v1` | One unit of work: parent, root, depth, creator, assignee, objective, budget, state | A child's budget is carved from its parent's remaining budget; depth ≤ `max_depth`; an assignee may not be an ancestor (no cycles) |
| `agent-event/v1` | One append-only audit event, hash-chained by `prev` | `prev` is the previous event's `sha256` (null only at `seq` 1). `generation` says who wrote any words in `data` |
| `agent-memory/v1` | One memory entry: scope (`agent`, `task`, `team`), the content's digest and size, provenance, what it supersedes | Content is stored apart from the record so it can be forgotten while the record's tombstone remains. Reads are filtered by scope by the host |
| `agent-approval/v1` | A request for a person's decision on one exact action | `action_sha256 = SHA-256(canonical(action))`. A decision is valid only for that digest, by `person`, before `expires_at`. Agents have no way to decide |

`generation` is always one of `model` (a language model wrote it),
`scripted` (fixed text or a rule-based policy), `person`, or `host` (code the
host runs). A system never relabels another system's generation.

## 3. Optional interop between James and the Lab

The Lab serves these routes **only** when its operator configures a token for
the scope, and only to loopback peers. Neither application needs the other.

| Route | Scope | Response |
| --- | --- | --- |
| `GET /api/rain/interop/v1/status` | `read` | `rain-interop-status/v1` |
| `GET /api/rain/interop/v1/evidence?episode=<id>` | `read` | `rain-interop-evidence/v1` |
| `POST /api/rain/interop/v1/proposals` | `propose` | `rain-interop-receipt/v1` |
| any failure | — | `rain-interop-error/v1` with an HTTP status |

Authentication is `Authorization: Bearer <token>`, one token per scope
(`RAIN_INTEROP_READ_TOKEN`, `RAIN_INTEROP_PROPOSE_TOKEN` in the Lab's server
environment), compared in constant time. A scope with no token is off. This
is a shared secret on loopback, **not** identity.

Least privilege, enforced separately on each side:

- `read` returns only records the Lab has already sealed. It runs nothing.
- `propose` stores a `rain-interop-proposal/v1` in the Lab's review queue.
  The receipt always says `"authorizes_execution": false`. Nothing submitted
  this way is authorised, preregistered or executed; a Lab operator reviews it
  with the Lab's own ritual or discards it.
- James sends a proposal only after its own approval gate: the person
  approves the exact proposal (its `action_sha256` is in `approval`). The
  Lab does not trust that field; it records it.
- Neither side exposes a route that mutates the other's agents, memory,
  tasks, registry, simulators, or any persistent organism.

Request bodies are at most 16 KiB; evidence responses at most 256 KiB.
James recomputes `rain-interop-evidence/v1`'s digest with the canonical rule
and refuses a body whose digest does not match.

## 4. Versioning

A change to a field, an enum or a rule is a new version (`agent-interop/v2`)
in a new directory. v1 stays as it is so old records keep their meaning.
