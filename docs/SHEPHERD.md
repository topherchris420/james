# Shepherd coordination in James (developer branch)

James includes an **optional, read-only** native Shepherd panel at **Shepherd**
(shortcut `Ctrl+6`, or `Command+6` on macOS). It connects to a locally
self-hosted [Shepherd](https://github.com/Korso-AI/Shepherd) hub and displays
agent presence, work claims and announcements.

This is **not** an AI agent orchestrator, MCP client, or replacement for
Shepherd. The actual coding agents still use Shepherd's MCP integration in
their respective clients. James is an observing wallboard.

## Setup

1. Set up Shepherd's **hub and PostgreSQL** using its
   [self-host instructions](https://github.com/Korso-AI/Shepherd).
   Shepherd uses Node.js, Fastify and PostgreSQL; its development scripts
   expect Node 22.9 or later when loading `.env` automatically.
2. Configure the hub's `DATABASE_URL`, a **strong random** `TEAM_TOKEN`,
   `ALLOWED_WORKSPACE` and (optionally) `HUB_PORT` in its private
   `.env` file. Do not commit that file or the token.
3. Start Shepherd's hub **on this computer**; the default `HUB_PORT` is
   `8080`. Ensure its `GET /health` endpoint is responding before connecting
   James. If another service uses 8080, move Shepherd to a free port.
4. Add Shepherd MCP to each coding agent separately using
   [Shepherd's client quickstart](https://github.com/Korso-AI/Shepherd/blob/main/docs/shepherd-mcp-quickstart.md).
   Link the repositories you want those agents to coordinate in.
5. Open James → **Shepherd**, enter the local hub port and **TEAM_TOKEN**,
   then press **Connect**. James fetches `GET /workspace/landscape`
   immediately and periodically while connected. **Refresh now** forces
   another read; **Disconnect and clear token** empties the in-memory token
   and removes the snapshot.

The dashboard reports three kinds of **claims made by agents**: who is active,
what paths they say they are changing, and what they announce. These are not
proof of committed files, successful tests or correct results.

## Explicit boundaries

- James makes **GET only** requests to
  `http://127.0.0.1:<port>/workspace/landscape` (or IPv6 loopback).
  It does not run npm, an MCP server or agent processes, and cannot reach a
  remote Shepherd hub. No additional network client is introduced.
- Connection is **off by default**. Polling happens only after Connect and
  stops after Disconnect. Errors clear the visible snapshot and back off
  rather than displaying stale data as current.
- James **does not save** the token, agent status or announcements to
  `session.json` or any conversation history, and never gives them to a model.
  The token is kept in process memory while connected; it is not encrypted
  there. Restarting James requires entering it again.
- In Shepherd self-host mode, `TEAM_TOKEN` is a **full-authority workspace
  credential**, not a read-only token. James itself uses it **only for GET**
  but an attacker with a copy could act on your hub. Restrict access to the
  hub and use a strong token. Prefer a dedicated, locally exposed service
  rather than a public, plaintext hub.
- Existing James desktop approval and R.A.I.N. Lab boundaries remain
  unchanged. Neither Shepherd messages nor work claims authorize an action.
- The feed is bounded to 512 KiB per response and limited in the UI to 80
  agents, 120 tasks and 60 announcements, with bounded text rendering.
- James does not include, link or modify Shepherd's AGPL source code;
  it talks to Shepherd's published JSON API as a separately deployed service.

## Current support and verification

The new Rust module contains fixture tests for the authenticated GET route,
JSON parsing, invalid credentials and clearing state on disconnect. Those
checks do **not** prove live interoperability with a deployed Shepherd hub,
OS-level secret protection, or the correctness of another agent's reports.
See [platform support](PLATFORM_SUPPORT.md) for CI status.

Future options: hosted-hub transport with explicitly reviewed network
permissions, a dedicated least-privilege dashboard credential, and
independently reviewed work-claim/announcement actions. None is enabled yet.
