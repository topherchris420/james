# Windows capabilities

All public operations are synchronous and belong on a worker thread. Each takes
a cloneable `Cancellation`; calling `cancel()` stops the helper process, including
speech playback and microphone recognition. Use a new handle per operation.

* `capture_screen` captures only the primary monitor, returning PNG bytes and
  pixel dimensions in memory. Capture must be initiated by the user. No file or
  clipboard is used. Protected windows and secure desktops may be blank.
* `dictate` recognizes one utterance using Windows System.Speech, with a 1–60
  second requested listening timeout and a further 10-second process startup
  allowance. It requires an installed Windows desktop speech recognizer, microphone,
  and Windows microphone permissions. This is push-to-talk, not a wake-word service.
* `speak` uses the installed Windows voice and stops when cancellation is requested.
* `execute` consumes one `ReviewedAction`. Show the exact action and destination
  coordinates/text to the user and call `approve` only from their confirmation
  handler. Never give a model access to `approve`. A review cannot be reused or
  altered after approval. Primary monitor dimensions are rechecked at execution.

Click, a small allowlist of individual keys, and 1–1000 printable text characters
are supported. No shell commands, shortcuts, action sequences, clipboard changes,
or automatic execution are exposed. SendKeys metacharacters are escaped. Input
goes to the foreground application; callers must provide a visible countdown to
let the user focus their target before executing a reviewed key/text action.
Review does not guarantee the destination window will remain unchanged; avoid
using this API for unattended workflows. Stop cannot undo an action already sent.

Helpers use Windows PowerShell, System.Drawing, System.Windows.Forms, System.Speech,
and a small C# user32 mouse interop definition. Rust contains no unsafe code.
Scripts are embedded constants, passed as encoded commands; all variable input
is JSON through stdin, never shell interpolation. Screen/audio data is never
persisted by this crate. Input cannot control elevated windows from an unelevated
James process. Other operating systems return `Error::Unsupported`.

The helper executable is resolved from the absolute Windows `SystemRoot`, not
the command search path. Output is capped at 64 MiB (PNG JSON) and 64 KiB (errors).
Normal unit tests never capture screens or activate microphone/speaker/input.
Two Windows-only ignored smoke tests opt into capture and voice-helper
cancellation: `cargo test -p james-platform -- --include-ignored`. They never
click, type, save screenshots, or send captured data anywhere. Run these only on
an authorized interactive Windows desktop.
