# Desktop reference research

Date: 2026-10-11. Reference: https://korsoai.com/ . Existing James baseline: `76e7e6a`.

REA `inspect_web_page` observed the public landing page using an isolated headless Chrome target. Evidence ID: `ev_26b675b7938a3bf09f494fcd060d3a8a7720179255a14c3308338dc10c35d0d9`. Discovery: `ev_4fa5fb1e491360fe8523d40a0a07fb813eb82dc806122a282c3a1bf3aec2f9f7`.

Observed: the page describes selecting screen content, visual guidance, voice dialogue and Windows availability. Its examples and FAQ describe user review of consequential operations and capture on request. These are public product representations, not independently verified desktop execution behavior.

James design choices: an original native Rust interface, explicit primary-monitor capture preview and vision attachment, one allowlisted action with a concrete review, native Windows dictation/read-aloud, an independently verified Lab API adapter, and per-user installer packaging. These implement the requested categories; they do not claim feature parity with Korso or copy its interface/assets.

Coverage: one public page, one 1-second passive observation window, same-origin DOM/accessibility/script metadata. No script source recovery, desktop package execution, backend observation, private API access, or proof of voice/control internals. Prior network activity was unavailable and cross-origin resources were filtered. Complete Evidence is held in the task work directory outside the source repository.

Implementation protocol references: [LM Studio OpenAI compatibility](https://lmstudio.ai/docs/developer/openai-compat), [Microsoft speech recognition](https://learn.microsoft.com/en-us/dotnet/api/system.speech.recognition.speechrecognitionengine.recognize?view=netframework-4.8), and [actual Lab contract](LAB.md).
