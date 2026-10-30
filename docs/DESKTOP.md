# James desktop 0.4

James is a native Rust desktop app (egui) for talking to a model that runs
on your own computer, with screen questions, one-at-a-time reviewed input on
Windows, voice, and a read-only view of the R.A.I.N. Lab. Release 0.4.0 is a
**developer preview**: [END_USER_READINESS.md](END_USER_READINESS.md) says
why, and [PLATFORM_SUPPORT.md](PLATFORM_SUPPORT.md) says how far each feature
has been checked on each system. For a plain-language guide, see
[GETTING_STARTED.md](GETTING_STARTED.md). This page is the reference for how
the app behaves.

## Install or build

Download a package from the [Releases page](https://github.com/topherchris420/james/releases):
a per-user Windows installer or portable `.exe`, a universal `James.app` for
macOS 11 and newer, or a `.deb` and archive for 64-bit Linux. No download
includes a model server, model weights or the Lab, and builds are unsigned.
[INSTALLER.md](INSTALLER.md) explains how each is built and checked.

From source, with Rust 1.88 or newer and native build tools (Visual Studio
C++ Build Tools on Windows):

```sh
cargo test --workspace --all-targets --locked
cargo run -p james-desktop --locked
```

`james --version` prints the version without opening a window.

## Layout

The sidebar has five pages: **Conversation** (Ctrl+1), **Screen context**
(Ctrl+2), **Computer control** (Ctrl+3), **R.A.I.N. Lab** (Ctrl+4) and
**Settings** (Ctrl+5); on a Mac, ⌘ replaces Ctrl. The title bar shows the
model and the app serving it, for example "gemma3:4b · Ollama", or
"… · runs elsewhere" in the warning colour when the server says the model is
remote. Clicking it opens Settings. F1 lists every shortcut.

## First run

Until setup is finished, Conversation shows a five-step guide. Progress is
saved after every step, so closing James resumes where you left off.

1. **Welcome**: what James is and what it never does on its own.
2. **Model app**: instructions for LM Studio, Ollama or llama.cpp written for
   the system James is running on, with copyable commands. James never
   installs or starts a model app and never downloads a model.
3. **Connect**: **Find my model server** asks LM Studio's, Ollama's and
   llama.cpp's usual ports on this computer (1234, 11434 and 8080) at once,
   only when pressed. It chooses a chat model over an embedding model, never
   chooses a model its server reports as running elsewhere, and never swaps
   a working model for a server that has none. When nothing answers, or a
   server answers without models, James says which and what to do next.
4. **Say hello**: a short test message to the chosen model, timed, so you
   know it works before relying on it. The test reply is not kept.
5. **Screen and safety**: what screen questions send, the permission your
   system needs (macOS Screen Recording, a Linux screenshot tool), an
   optional practice screenshot that is never sent, what clicks and typing
   mean on this system, and where conversations are kept.

**Skip setup and use Settings** leaves the guide at any step, and
**Settings → Run guided setup again** restarts it.

## Models

| Model app | Requests James makes |
| --- | --- |
| LM Studio | `/v1/models`, `/api/v0/models`, `/v1/chat/completions` (server-sent events) |
| Ollama | `/api/tags`, `/api/show`, `/api/ps`, `/api/chat` (newline-delimited JSON) |
| llama.cpp `llama-server` | `/v1/models`, `/props`, `/v1/chat/completions` (server-sent events) |
| Other OpenAI-compatible servers | `/v1/models`, `/v1/chat/completions` |

**Settings → Local model** chooses the app and port, lists its models with
**Discover models**, and runs **Find my model server**. **Check what this
model can do** asks the server whether the model reads images, can chat, how
large its context window is and whether it is loaded, and shows the answer
"according to" that server. James never guesses from a model's name: what a
server doesn't say is shown as not stated. When the model can't read images,
the controls that would send a screenshot are disabled with the reason.

All model and Lab traffic goes through `james-loopback`, whose only address
input is a port on `127.0.0.1` (or `::1`): no DNS, redirects or proxies, and
no other HTTP client exists in James. That proves the *connection* stays on
this computer, not the *model*. Ollama can relay to its cloud service; James
labels models the server reports as remote ("runs elsewhere"), warns before
you use one, and never selects one automatically. Other servers may relay
without saying so.

## Conversation

Write in the message box and choose **Send** or press Ctrl+Enter. Replies
appear as the model writes them and are shown as Markdown: headings, lists,
emphasis, code, quotes and tables. Links always show their full address.
Clicking one opens a dialog that names the site, shows the address and warns
when it carries a lot of text after the site name; the link opens in your
browser only when you choose **Open in browser**. Reasoning that a model writes in a leading `<think>`
block is folded away under "Model's reasoning (not checked)"; while a model
reasons separately (Ollama's thinking models), James shows that it is
thinking and keeps only the answer. Reasoning is never read aloud or treated
as an action.

Each message has **Copy** and each answer **Read aloud**. The latest answer
also has **Ask again**, which sends your last question again and replaces
that answer (a screenshot is not sent again). Ctrl+Shift+C copies the last
answer and Ctrl+L jumps to the message box.

**Copy conversation** above the messages copies the chat as Markdown, with
You and James headings and the original message formatting. Screenshots,
the unsent draft and internal instructions are excluded. **New conversation**
(Ctrl+N, or ⌘N on Mac) asks before clearing the messages, draft, screenshot,
attachments and unapproved action proposal, then focuses the message box.
Settings and unresolved approved-action records stay. Both controls remain
above the scroll area and are disabled while a task is running. Retrying an
answer preserves your unsent draft and its screenshot attachment; that
screenshot is not sent with the retry.

**Stop** (or Esc) closes the connection to the model server at once, even
before the first word. The part of the reply that arrived stays, marked
"[Stopped by you before the reply finished.]"; a reply cut short by an error
is kept the same way with the reason. Whether the server stops generating is
up to the server: Ollama 0.40.3 was observed to cancel generation. James
waits up to two minutes for each next piece of a reply and 30 minutes for a
whole reply.

**What is sent.** An ordinary question carries up to the 30 most recent
messages. When the server reports the model's context window, history is
trimmed to fit it with room for the reply; otherwise it is capped at about
160 KB. James says when older messages were left out. Guide me and Suggest
one action send no history and no Lab snapshot, only the screenshot and the
question.

**Retention.** James keeps at most 80 recent messages within 512 KiB of text.
A message over 32 KiB is shortened at a character boundary and marked as
shortened, rather than discarded. This also bounds the saved JSON below its
4 MiB read limit, even when text contains characters JSON must escape.
Oversized settings are refused without replacing the previous saved file.
Turning history off is enforced by the file writer itself: conversation
text is omitted from the saved file.

If a save fails, a warning remains above every page until a later save
succeeds. **Retry saving** writes the current conversation and settings
again. Clearing a conversation and switching history off report success
only after the file was written.

**When something goes wrong**, James names the cause in plain words with a
next step: the model app isn't running, the model isn't installed or
loaded, the conversation no longer fits the context window, the model can't
read images, it isn't a chat model, the model needs a sign-in (Ollama cloud
models), the server went silent or took too long, or the server sent
something James couldn't read. Settings links to the troubleshooting entry
for the last error.

## Screen questions and Guide me

**Screen context → Capture primary screen** hides James for three seconds so
you can bring the right window forward, then takes one picture of the
primary monitor and shows it. It is kept in memory only. On Windows, James
also records which window was in front. Tick **Attach this screenshot to my
next question** to send it with your next message. A screenshot more than a
minute old is flagged and can't be used to review an action.

**Guide me** (under the message box) captures the screen and asks the model
what matters on it and up to three next steps you can take yourself. If the
message box holds a question, that is asked instead. The answer is guidance
only and never becomes an action.

## Computer control (Windows)

On Windows, James can send **one** click, key press or piece of typing at a
time, only after you approve exactly that input:

1. **Propose.** Click a point in the screenshot, prepare a key or text on
   Computer control, or press **Suggest one action** to let a vision model
   propose one. Every proposal must be exactly one click inside the
   screenshot, one listed key, or up to 1,000 characters of visible text on
   one line: no line breaks, and no invisible formatting characters such as
   zero-width or text-direction marks. A model's reply is refused before review if it holds several
   actions, text around the JSON, duplicate fields, shell commands, key
   chords, look-alike letters, coordinates that aren't plain pixels, or a
   claim of approval.
2. **Review.** Computer control shows the exact input (typing in full, in a
   scrollable box), the window it is meant for, the reason (flagged, and
   shown on one line, when a model wrote it), and the screenshot with the
   click point marked. When a model proposed a click, typing, or Enter,
   Space, Delete or Backspace, tick **I checked what this does and where it
   goes** first. Choose **Allow once** or **Deny**.
3. **Countdown.** Allow once starts a five-second countdown so you can
   switch to the target window. **Cancel (Esc)** stops it. There is no
   system-wide stop key: Esc works only while James's window has focus.
4. **Checks before sending.** The approval expires if the countdown ends
   more than two seconds late, the wall clock moves backwards, or the
   screenshot passes 60 seconds. The Windows helper then refuses to send
   anything if James itself is in front, if the window in front is not the
   one recorded at capture (same window and process), or if a click would
   land in a different window. Each refusal says that nothing was sent.
5. **Intent first.** Before any input, James writes the exact action, the
   target window and the time to `action-intent.json` and flushes it to
   disk. If that fails, nothing is sent.
6. **One input, then a check.** After sending, James records the outcome
   and blocks further actions until you look at the app and choose **I
   checked the app — continue**. A crash, power loss or forced quit leaves
   the record in place: on the next launch James shows what was approved,
   when, and that the outcome is unknown. It never repeats an action by
   itself.

Approval is for one input only. Voice can never approve, and a model's
words can never approve or chain anything. James does not sandbox the
target application, verify that the input had the intended effect, or
support several monitors. A click or Enter can still submit, send or delete
something, and sent input can't be undone.

On macOS and Linux, James never clicks or types: Computer control explains
this and points to Guide me. If an unresolved action record is ever present
in the data folder, every system shows it for checking. Secure input on
macOS and Linux needs its own reviewed design, which does not exist yet.

## Voice

**Talk** holds a hands-free conversation: James listens, asks the model,
speaks the answer and listens again. Saying just "stop", "goodbye" or
"that's all", pressing Esc or **End conversation**, an error, or 15 seconds
of silence ends it. **Dictate** puts one utterance into the message box for
you to check. **Read aloud** speaks an answer, and **Settings → Voice → Read
new answers aloud** speaks each new one. Markdown, code blocks and links are
cleaned up before speaking. Voice requests are chat only: voice can never
approve or send input.

- **Windows** uses the installed Windows speech recognizer and voices.
- **macOS and Linux** listen with [whisper.cpp](https://github.com/ggml-org/whisper.cpp)
  on this computer. A recorder (`sox` or `ffmpeg` on macOS; `parec`,
  `arecord`, `sox` or `ffmpeg` on Linux) streams audio to James, which
  starts an utterance after 120 ms of voice above the room's noise floor and
  ends it after a 1.2-second pause (30 seconds at most). Each utterance is
  written to a private temporary folder, transcribed by `whisper-cli` and
  deleted at once. Speech uses `say` on macOS and `espeak-ng` or `spd-say`
  on Linux.

**Settings → Voice** shows whether listening is ready and gives copyable
setup commands; James never downloads speech models itself. Talk and
Dictate were not exercised in 0.4.0 testing (no microphone was available).

## R.A.I.N. Lab

**R.A.I.N. Lab** is a read-only view of a Lab running on this computer.
**Find the Lab** checks ports 5173, 5174, 5175 and 4173; **Connect** uses
the port you enter. While connected, James refreshes every five seconds and
backs off to once a minute while the Lab doesn't answer. The page explains
the current stage in words, lists recent simulator results with how far
each was checked, and counts what the Lab knows by its standing. **Share a
Lab snapshot with my next questions** adds a short, labelled summary to each
chat question, never to Guide me or Suggest one action; **Exactly what James would share with the model** shows that text
in full. A snapshot older than 15 seconds is never sent. James never
starts, steers or approves anything in the Lab. See [LAB.md](LAB.md).

## Appearance and accessibility

**Settings → Appearance and accessibility** offers **Match my computer**,
**Light** or **Dark**, **High contrast**, **Text and controls size** from
80% to 200% (also Ctrl+plus, Ctrl+minus and Ctrl+0), and **Reduce motion**
(no animations, spinners or smooth scrolling). Every control is reachable
with Tab and Shift+Tab and pressed with Space or Enter, with a visible focus
ring. James publishes its controls, labels and messages to the system
accessibility interface through AccessKit (UI Automation, NSAccessibility,
AT-SPI). This has been checked by automated tests that drive the app through
that tree, **not with a real screen reader**.

## Your data

James keeps everything in one folder: `%LOCALAPPDATA%\Vers3Dynamics\James`
on Windows, `~/.james` on macOS and Linux.

| File | Contains | Kept until |
| --- | --- | --- |
| `session.json` | Settings, setup progress and, if you keep history, the conversation | You change or delete it |
| `action-intent.json` | The last approved action, including any text to type, and its outcome | You confirm you checked the app |
| `crash.log` | Version, system, thread, error location and backtrace; no conversation text | You delete it from Conversation |

On macOS and Linux the folder is owner-only (0700) and its files 0600,
written atomically; older installs are tightened on first launch. On
Windows, James relies on the default per-user permissions of
`%LOCALAPPDATA%`, which it neither changes nor checks. Files are **not
encrypted**; full-disk encryption protects them if the computer is lost.

**Settings → Privacy and your data** shows this list and the folder.
**Keep my conversations on this computer** can be turned off, which removes
the saved copy. **Clear conversation…** and **Delete all James data…** ask
first; deleting everything keeps an unresolved action record until you have
checked it. Deleting is not secure erasure.

Screenshots are never saved by James. Where a system tool needs a file
(macOS and Linux capture, whisper.cpp), it lives in a private temporary
folder and is deleted at once. What you send, attached screenshots and Lab
snapshots go to your model app, which may keep its own logs.

The Windows uninstaller asks whether to delete this folder (the default is
to keep it, and a silent uninstall never deletes it). Removing the Linux
package or running the archive's `install.sh --uninstall` keeps it.

## Crash reports

If James panics on any thread, it writes `crash.log` before exiting or
stopping that task. On the next launch, Conversation offers a link to report
the problem and a button that deletes the report. Nothing is sent anywhere.

## Updates

James never checks for updates by itself, because it never connects to the
internet. **Settings → Help** shows the installed version and opens the
releases page in your browser when you choose. Each release lists SHA-256
checksums and carries build-provenance attestations; see
[INSTALLER.md](INSTALLER.md#check-a-download).
