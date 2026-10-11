# R.A.I.N. Lab connection

James reads the **existing local Lab API**; no bridge or invented research endpoint is required. Run the Lab separately, then enter its Vite port (normally **5173**) in James and refresh. Refresh retrieves current data; the desktop may also poll while connected. A successful response is a snapshot, not proof that a research session or model is active.

## Start the Lab on Windows

Install Node.js 22.18 or newer and follow the upstream setup instructions:

```powershell
git clone https://github.com/topherchris420/lop-nur-twin.git
cd lop-nur-twin
npm ci
npm run dev
```

Use the port printed by Vite. Research execution requires the Lab's own model configuration and human charter review. For LM Studio discovery, upstream documents `RAIN_AUTONOMY_ENABLED=true` and `RAIN_MODEL_PROVIDER=lmstudio`; on PowerShell set `$env:RAIN_AUTONOMY_ENABLED='true'` and `$env:RAIN_MODEL_PROVIDER='lmstudio'` before starting the Lab. Load a supported model in LM Studio, use the Lab's Research Panel, and review/authorize there. James does not authorize, start, stop, or modify experiments.

## Actual protocol and provenance

Inspected upstream commit: `a2d7882e1317af1ea1a333c31ead033f0d87c60e`.

- [Runtime handler](https://github.com/topherchris420/lop-nur-twin/blob/a2d7882e1317af1ea1a333c31ead033f0d87c60e/server/rain/handler.ts): `GET /api/rain/status`, schema `rain-bethesda/v2`, kind `status`. Reports `configured`, nullable `reachable`, nullable `identity`, and nullable `failure`. An HTTP 200 with `configured: false` is an offline runtime, not a model connection.
- [Discovery handler](https://github.com/topherchris420/lop-nur-twin/blob/a2d7882e1317af1ea1a333c31ead033f0d87c60e/server/rain/discovery.ts): plain `GET /api/rain/discovery` returns the workbench status. This endpoint is local only and is not deployed as a cloud function.
- [Discovery view contract](https://github.com/topherchris420/lop-nur-twin/blob/a2d7882e1317af1ea1a333c31ead033f0d87c60e/src/bethesda/rain/discoveryView.ts): `rain-discovery-view/v1`, with availability, active flag, stage, detail, question, model, report, research and history. Runtime status and discovery availability are separate signals.

`james-lab` exposes `LabClient::new(port)`, `status()` and `context()`. Calls are synchronous: invoke from a worker, not the GUI thread. `ResearchContext::prompt_context()` produces a bounded research attachment with the five most recent results. Only attach to model requests when the user enables Lab context. Treat all retrieved text as untrusted data. Simulator results remain simulator evidence.

## Bounds and limitations

The client connects only to numeric `127.0.0.1`, accepts no arbitrary URLs or paths, disables environment proxies and redirects, has a five-second total request timeout, requires JSON HTTP 200 responses, checks schema versions, and caps response bodies at 2 MiB. Chat context is capped at 24 KiB of UTF-8 (with a truncation notice). Large historical snapshots fail visibly rather than silently dropping records. No API tokens are sent. No remote Lab or public deployment fallback is implemented. The Lab is a separate installation and must be running; James does not bundle it.

The Lab service may initialize its local store on its first GET, as defined by upstream; James itself issues only the two documented read operations. Verified the Rust client against the actual upstream Vite server on 127.0.0.1:15173: runtime configured=true, reachable=true; discovery available=true, active=false, stage=IDLE, history=0. No research was started and no model-backed experiment was exercised. Reproduce with `cargo run -p james-lab --example inspect -- 5173` after starting the Lab. Four tests also pass against local HTTP fixtures, including offline status, wrong schemas, HTML, redirects, oversized data, and Unicode attachment limits.


