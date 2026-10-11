# Desktop capabilities

All public operations are synchronous and belong on a worker thread. Each takes
a cloneable `Cancellation`; calling `cancel()` stops the helper process, including
speech playback and microphone recognition. Use a new handle per operation.
`listening(&VoiceConfig)`, `can_control_input()`, `uses_whisper()` and
`voice_support()` tell the UI what the current platform supports so it can
disable what is unavailable and explain why.

| Operation | Windows | macOS | Linux |
| --- | --- | --- | --- |
| `capture_screen` | PowerShell + System.Drawing, primary monitor | `screencapture -x -m` (main display) | `grim` (wlroots Wayland), `gnome-screenshot`, `spectacle`, or ImageMagick `import` (X11) |
| `speak` | System.Speech installed voice | `say` | `espeak-ng`/`espeak`, else `spd-say` |
| `dictate` | System.Speech dictation | whisper.cpp, mic via `sox` or `ffmpeg` | whisper.cpp, mic via `parec`, `arecord`, `sox` or `ffmpeg` |
| `execute` | One reviewed click/key/text | `Error::Unsupported` | `Error::Unsupported` |

* `capture_screen` returns PNG bytes and pixel dimensions in memory and must be
  initiated by the user. No clipboard is used. Where a tool can only write a
  file (macOS, gnome-screenshot, spectacle), the capture goes to a fresh
  owner-only (`0700`) temporary folder and is deleted immediately after being
  read. Protected windows and secure desktops may be blank. macOS requires the
  Screen Recording permission.
* `dictate` recognizes one utterance; the 1–60 second timeout is how long to
  wait for speech to begin. It is push-to-talk, not a wake-word service, and
  returns an empty string when nothing was said. On macOS/Linux, a recorder
  streams raw 16 kHz mono PCM over a pipe into `listen::capture`, an
  energy endpointer measured in audio time: the noise floor is the quietest
  30 ms frame of the last 1.5 s, speech starts after 120 ms above three times
  that floor (taps of 60 ms or less never qualify), the threshold freezes once
  speech starts, and a 1.2 s pause or 30 s cap ends it. Only detected speech
  reaches `whisper-cli` (`-nt -np`, English for `*.en` models), via a WAV in a
  private folder that is deleted immediately. `[BLANK_AUDIO]`-style annotations
  are removed. A recorder that yields no audio within 4 s is skipped for the next.
  `VoiceConfig` can name the `whisper-cli` program and model explicitly
  (absolute paths only); otherwise both are found automatically.
* `speak` uses an installed system voice and stops when cancellation is
  requested. (`spd-say` hands text to speech-dispatcher, which may finish the
  current sentence after cancellation.)
* `execute` consumes one `ReviewedAction`. Show the exact action and destination
  coordinates/text to the user and call `approve` only from their confirmation
  handler. Never give a model — or a voice command — access to `approve`. A
  review cannot be reused or altered after approval. Primary monitor dimensions
  are rechecked at execution.

Click, a small allowlist of individual keys, and 1–1000 printable text characters
are supported. No shell commands, shortcuts, action sequences, clipboard changes,
or automatic execution are exposed. SendKeys metacharacters are escaped. Input
goes to the foreground application; callers must provide a visible countdown to
let the user focus their target before executing a reviewed key/text action.
Review does not guarantee the destination window will remain unchanged; avoid
using this API for unattended workflows. Stop cannot undo an action already sent.

## Process safety

Every helper runs under one shared supervisor with a deadline, cancellation, and
output caps of 64 MiB (stdout) and 64 KiB (stderr). Executables are never found
through the search path: Windows PowerShell is resolved from the absolute
`SystemRoot`, and macOS/Linux tools only from fixed system directories
(`/usr/bin`, `/bin`, `/usr/local/bin`, `/opt/homebrew/bin`,
`/run/current-system/sw/bin`). Windows scripts are embedded constants passed as
encoded commands with all variable input as JSON on stdin. On macOS/Linux, text
to speak travels over stdin (or as a single argument after `--` for `spd-say`).
Nothing passes through a shell. Rust contains no unsafe code. Input cannot
control elevated windows from an unelevated James process.

Normal unit tests never capture screens or activate microphone/speaker/input.
Ignored smoke tests opt into screen capture (Windows, macOS, Linux) and voice
helper cancellation (Windows): `cargo test -p james-platform -- --include-ignored`.
They never click, type, save screenshots, or send captured data anywhere. Run
these only on an authorized interactive desktop.
