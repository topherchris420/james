# Request evidence and source references

Available on the development branch; not in the published preview download.

Open **Add context** beside **Send** to attach a text file, an extractable PDF,
or selected workspace files. The composer shows what goes with the question
and lets you remove it before sending. In a narrow Companion window, open the
**Context** menu to see the attached items. James numbers the extracted text
and asks the model to cite locations such as `[S1:L2]` and `[S2:L4-L7]`.

Under the latest document answer, **Check the sources** lists recognized
references. Expand one to read the actual supplied lines. Unknown source IDs,
malformed references and nonexistent or reversed ranges are marked unresolved.
Up to the first 64 `[S…` attempts are inspected, with duplicate references
shown once, to bound work on untrusted model output. An answer with no
recognized references in that scan says so. A reference that locates real lines does
**not** establish that those lines support the claim, that the document is true,
or that the model has cited every factual claim. Check the text yourself.

## Inspect a request

Choose **Request evidence** in the conversation toolbar (**Options** in a narrow
Companion window), or **Inspect request and sources** below a document answer.
The window shows:

- Model, provider and local port at the time of the request.
- Prepared text messages, including instructions, included conversation history,
  selected sources, any opted-in Lab context, and the question.
- Source snapshots with line numbers, text byte counts and omitted-history count.
- Whether an image or screenshot region was selected; image pixels are excluded
  from this record. The existing image processing may crop, annotate and scale it.
- Pending, complete, incomplete or failed status, elapsed time and an interruption
  or error note when applicable. Completion means the stream ended normally,
  not that the answer is correct.

The record describes the **prepared request**. Provider adapters may join system
messages or encode images differently. A failed request may never reach the
server. James cannot prove the server processed the request or kept it local.

**Copy evidence JSON** explicitly copies the record, source text, answer and
reference results to the clipboard using schema `james-request-evidence/v1`.
This is useful for privately comparing models or documenting a failure. Review
it before sharing: it contains prompts and potentially sensitive document text.
It is not a signed attestation or a benchmark of model accuracy.

## Source scope and retention

Only the latest chat request has a record. It is held in process memory and is
never added to `session.json`, archived conversations or crash reporting by this
feature. It is replaced by the next successfully prepared chat request and
cleared by a new guidance/action-proposal request, conversation change, deleting
all data, **Forget request evidence**, or exit. A rejected preflight leaves the
previous record intact. There is no secure-memory-erasure guarantee.

The model's answer may quote a source; that answer is ordinary conversation text
and follows your history setting. **Forget** removes the request record, not
the answer. Clipboard managers and model-server logs have separate retention.

Source IDs are local to one request. PDF line numbers refer to extracted text,
not PDF page numbers. Screenshots and Lab snapshots are not line-cited sources.
Documents remain single-question attachments: **Ask again** does not restore
previous documents or images. Reattach them for a source-based follow-up. Old
conversations may still display citation text, but their source snapshots are
not retained and are not presented as verified references.

## Context limits

James refuses a document if its JSON-encoded, numbered source message exceeds
32 KiB. It does not silently cut selected source text. When the model advertises
a context window, James checks fixed text against its existing heuristic:
three bytes per token, reserving a quarter of the window (up to 2,048 tokens) for
a reply. If the question plus fixed context alone exceeds that estimated budget,
James asks for shorter context or a larger-window model before launching a
request. Drafts and selections remain intact.

This is **not exact token counting** and does not account for image tokens.
Languages and model tokenizers differ. An unknown context window skips this
estimate; transport and per-message bounds still apply. Older conversation
messages are trimmed to the remaining budget (also capped at 240 KiB of
prepared request text to stay below the transport limit) and the record reports how many
were omitted.

## Find earlier work

Open **History** and type in **Search conversation history**. Search runs only
against retained titles and messages on this computer. It is case-insensitive,
uses literal words (all words must match somewhere), and makes no model or
filesystem search request. Existing retention limits still apply.

## Validation scope

Automated fake-server and headless GUI tests exercise exact source transport,
provider system-message coalescing, invalid and missing references, Unicode and
blank lines, oversized-request refusal, completion/failure/interruption,
retention-index changes, forgetting, conversation changes, non-persistence and
history search. They test application behavior, not whether a real model follows
the citation instructions. Real local-model answer quality and physical Windows
and macOS usability still require independent trials.

Korso's [security page](https://korsoai.com/security), updated November 6, 2026,
states that its desktop app was discontinued November 3. Earlier Korso workflows
are a historical design reference; these changes make no claim of measured
product superiority or access to Korso's replacement product.
