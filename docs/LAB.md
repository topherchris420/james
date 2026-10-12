# R.A.I.N. Lab connection

James can show what a [R.A.I.N. Lab](https://github.com/topherchris420/lop-nur-twin)
on the same computer is doing, using the Lab's **existing local API**. No
bridge or extra endpoint is needed. The connection is read-only: James never
starts, steers, approves or stops anything in the Lab, and the Lab is
optional. James works the same without it.

The Lab is a simulator. Everything James shows from it is labelled as
simulator output, a model's proposal, or something the Lab observed. None of
it is real-world evidence, and none of it is an instruction to James or to
the model.

## Start the Lab

Install Node.js 22.18 or newer, then:

```sh
git clone https://github.com/topherchris420/lop-nur-twin.git
cd lop-nur-twin
npm ci
npm run dev
```

Vite prints the port it chose: normally **5173**, or 5174 or 5175 when that
is taken; `npm run preview` uses 4173. Running research needs the Lab's own
model configuration and the human review the Lab asks for. For LM Studio,
the Lab documents `RAIN_AUTONOMY_ENABLED=true` and
`RAIN_MODEL_PROVIDER=lmstudio` (in PowerShell,
`$env:RAIN_AUTONOMY_ENABLED='true'` and `$env:RAIN_MODEL_PROVIDER='lmstudio'`)
before `npm run dev`. All of that happens in the Lab, not in James.

## In James

Open **R.A.I.N. Lab** (Ctrl+4) and press **Find the Lab**, which checks ports
5173, 5174, 5175 and 4173 on this computer, or type the port and press
**Connect**. While connected, James refreshes every five seconds. When the
Lab stops answering, James says so and waits longer before each new attempt
(5, 10, 20, 40, then 60 seconds) so it never hammers a port. It keeps the
last snapshot on screen but never sends one that is more than 15 seconds
old: a question with sharing on is held back with an explanation instead.
**Disconnect** clears the snapshot and turns sharing off.

The page shows:

- **Lab build**: the commit the Lab reports, whether it has uncommitted
  changes, and whether its research meetings are scripted or written by a
  model (for example "lop-nur-twin 4871ef6 · scripted meetings").
- **Right now**: the stage in plain words, the current question, the Lab's
  note (labelled as possibly written by a model), and where the Lab saved a
  report. James shows the report's path; it does not read the file.
- **Recent simulator results**, newest first, at most five: the verdict
  ("Supported in the simulator", "Not supported in the simulator", "Not
  enough evidence in the simulator"), whether the run was **replayed and
  registered** in the Lab or **not confirmed by replay**, exploratory or
  confirmatory, the question, the hypothesis (labelled "proposed by a
  model"), the setting, the mean effect with its unit, and the run id.
- **What the Lab knows, by how far it's been checked**: counts of its
  knowledge graph by standing, most trusted first: verified in the
  simulator, observed by the Lab, assessed by a person, from reading
  sources, suggested by a model.

### Sharing with the model

Nothing from the Lab reaches the model unless you tick **Share a Lab
snapshot with my next questions**. Then each question carries a compact
snapshot of at most 24 KiB that starts by telling the model it is untrusted
research data, not instructions, and that simulator findings are not
real-world evidence. Every item in it is labelled (simulator run,
model-proposed hypothesis, detail that may be model-written). **Exactly what
James would share with the model** shows the full text before you send
anything. The snapshot never includes the report path or anything from your
files.

## Protocol and provenance

Checked against upstream commit
[`4871ef6314d5975d82f2ad94366a9dcae72b4241`](https://github.com/topherchris420/lop-nur-twin/tree/4871ef6314d5975d82f2ad94366a9dcae72b4241)
(2026-10-11):

- `GET /api/rain/status`
  ([handler](https://github.com/topherchris420/lop-nur-twin/blob/4871ef6314d5975d82f2ad94366a9dcae72b4241/server/rain/handler.ts),
  schema `rain-bethesda/v2` in
  [contracts.ts](https://github.com/topherchris420/lop-nur-twin/blob/4871ef6314d5975d82f2ad94366a9dcae72b4241/src/bethesda/rain/contracts.ts)):
  whether the runtime is configured and reachable, and its identity
  (commit, uncommitted changes, meeting generation, model). An HTTP 200 with
  `configured: false` means the Lab's runtime is off, not that a model is
  connected.
- `GET /api/rain/discovery`
  ([handler](https://github.com/topherchris420/lop-nur-twin/blob/4871ef6314d5975d82f2ad94366a9dcae72b4241/server/rain/discovery.ts),
  view schema `rain-discovery-view/v1` in
  [discoveryView.ts](https://github.com/topherchris420/lop-nur-twin/blob/4871ef6314d5975d82f2ad94366a9dcae72b4241/src/bethesda/rain/discoveryView.ts)):
  availability, stage, detail, question, report path, research programme,
  history and knowledge graph. This endpoint exists only on a local Lab.

James sends only these two requests, always without a query string. The
query forms (`?verify=`, `?artifact=`) can make the Lab replay runs or read
files, so James never uses them. The Lab may create its local store on the
first request, as upstream defines.

`james-lab` exposes `find_lab(ports)`, `LabClient::new(port)`, `status()`
and `context()`, and `ResearchContext::{findings, standing_counts,
prompt_context}`. Calls block, so the desktop makes them on worker threads.

## Bounds

Requests go through `james-loopback`: a numeric port on `127.0.0.1` only, no
URLs, DNS, proxies or redirects, a two-second connection limit and five
seconds in total. Responses must be JSON with HTTP 200 and the expected
schema, and a reply larger than 2 MiB is refused with a visible error
rather than silently trimmed. In the shared snapshot, Lab text is reduced to
single lines without control characters and each field is length-limited.
No tokens or credentials are sent, and there is no remote or cloud Lab.

## Verification

- **Real Lab** (2026-10-12): lop-nur-twin at `4871ef6` with Node 22.22,
  `npm ci` and `npm run dev`. **Find the Lab** found port 5173; the page
  showed "lop-nur-twin 4871ef6 · scripted meetings", the idle stage in
  words, no finished experiments, and 62 Lab-observed and 49 source items in
  the knowledge graph. No experiment was started, so the results list has
  only been exercised with recorded data. See [VALIDATION.md](VALIDATION.md).
- **Automated**: eight `james-lab` tests against local HTTP fixtures (the
  real discovery contract, an offline runtime, wrong schemas, HTML,
  redirects and oversized replies, how findings and the Lab build are
  labelled, a port search that skips other servers and gives up in time,
  and Unicode and size limits of the shared snapshot), and desktop GUI tests
  for the page and its offline back-off.
- **Reproduce**: start the Lab, then
  `cargo run -p james-lab --example inspect -- 5173`.
