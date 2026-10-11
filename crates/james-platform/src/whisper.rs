//! whisper.cpp listening for macOS and Linux, entirely on this computer.
//!
//! A system recorder streams 16 kHz mono PCM over a pipe; the endpointer in
//! `listen` decides when speech starts and stops, so silence is never sent to
//! whisper (which tends to invent text from silence). The utterance is written
//! as a WAV file into a private owner-only folder, transcribed by `whisper-cli`,
//! and deleted immediately. Nothing passes through a shell.
use crate::listen::{self, Capture, Limits};
use crate::sys::{find_tool, with_private_dir};
use crate::{Cancellation, Error, Listening, VoiceConfig, supervise};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

const PROGRAMS: &[&str] = &["whisper-cli", "whisper-cpp"];
const TRANSCRIBE_LIMIT: Duration = Duration::from_secs(180);
/// A recorder that produces no bytes this long after starting is skipped.
const NO_AUDIO_AFTER: Duration = Duration::from_secs(4);

/// One way to read the default microphone as raw s16le, 16 kHz, mono on stdout.
struct Recorder {
    tool: &'static str,
    args: &'static [&'static str],
}

const SOX: Recorder = Recorder {
    tool: "sox",
    args: &[
        "-q",
        "-d",
        "-t",
        "raw",
        "-r",
        "16000",
        "-e",
        "signed-integer",
        "-b",
        "16",
        "-c",
        "1",
        "-L",
        "-",
    ],
};

#[cfg(target_os = "macos")]
const RECORDERS: &[Recorder] = &[
    SOX,
    Recorder {
        tool: "ffmpeg",
        args: &[
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "avfoundation",
            "-i",
            ":0",
            "-ac",
            "1",
            "-ar",
            "16000",
            "-f",
            "s16le",
            "-",
        ],
    },
];

#[cfg(not(target_os = "macos"))]
const RECORDERS: &[Recorder] = &[
    // PulseAudio and PipeWire (via pipewire-pulse): most desktops.
    Recorder {
        tool: "parec",
        args: &[
            "--format=s16le",
            "--rate=16000",
            "--channels=1",
            "--latency-msec=30",
        ],
    },
    Recorder {
        tool: "arecord",
        args: &["-q", "-t", "raw", "-f", "S16_LE", "-r", "16000", "-c", "1"],
    },
    SOX,
    Recorder {
        tool: "ffmpeg",
        args: &[
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "pulse",
            "-i",
            "default",
            "-ac",
            "1",
            "-ar",
            "16000",
            "-f",
            "s16le",
            "-",
        ],
    },
    Recorder {
        tool: "ffmpeg",
        args: &[
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "alsa",
            "-i",
            "default",
            "-ac",
            "1",
            "-ar",
            "16000",
            "-f",
            "s16le",
            "-",
        ],
    },
];

fn recorder_hint() -> &'static str {
    if cfg!(target_os = "macos") {
        "No microphone recorder found. Install SoX (brew install sox) or ffmpeg."
    } else {
        "No microphone recorder found. Install pulseaudio-utils (parec), alsa-utils (arecord), sox or ffmpeg."
    }
}

fn program_hint() -> &'static str {
    if cfg!(target_os = "macos") {
        "whisper.cpp was not found. Install it with: brew install whisper-cpp — or set the program path in Settings."
    } else {
        "whisper.cpp was not found. Build it (github.com/ggml-org/whisper.cpp) or install your distribution's package, then set the whisper-cli path in Settings if James cannot find it."
    }
}

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
}

/// James's own models folder, where Settings tells users to put a model.
pub fn models_dir() -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        home().map(|h| h.join("Library/Application Support/James/models"))
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| home().map(|h| h.join(".local/share")))
            .map(|d| d.join("james/models"))
    }
}

fn model_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = models_dir().into_iter().collect();
    if let Some(home) = home() {
        dirs.push(home.join("whisper.cpp/models"));
    }
    dirs.push(PathBuf::from("/opt/homebrew/share/whisper-cpp"));
    dirs.push(PathBuf::from("/usr/local/share/whisper-cpp"));
    dirs.push(PathBuf::from("/usr/share/whisper.cpp/models"));
    dirs
}

fn explicit_file(path: &Path, what: &str) -> Result<PathBuf, String> {
    if path.is_absolute() && path.is_file() {
        Ok(path.to_path_buf())
    } else {
        Err(format!(
            "The {what} setting must be the full path to an existing file: {}",
            path.display()
        ))
    }
}

fn find_program(config: &VoiceConfig) -> Result<PathBuf, String> {
    if let Some(path) = &config.whisper_program {
        return explicit_file(path, "whisper.cpp program");
    }
    PROGRAMS
        .iter()
        .find_map(|name| find_tool(name))
        .or_else(|| {
            // The default location when built from source in the home folder.
            home()
                .map(|h| h.join("whisper.cpp/build/bin/whisper-cli"))
                .filter(|p| p.is_file())
        })
        .ok_or_else(|| program_hint().to_owned())
}

fn find_model(config: &VoiceConfig) -> Result<PathBuf, String> {
    if let Some(path) = &config.whisper_model {
        return explicit_file(path, "whisper model");
    }
    for dir in model_dirs() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let names: Vec<String> = entries
            .filter_map(Result::ok)
            .filter(|e| e.path().is_file())
            .filter_map(|e| e.file_name().into_string().ok())
            .collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        if let Some(name) = listen::choose_model(&refs) {
            return Ok(dir.join(name));
        }
    }
    Err(match models_dir() {
        Some(dir) => format!(
            "No whisper model found. Download one (for example ggml-base.en.bin) into {}. Settings shows the command.",
            dir.display()
        ),
        None => "No whisper model found. Set the model file in Settings.".into(),
    })
}

fn available_recorders() -> Vec<(PathBuf, &'static Recorder)> {
    RECORDERS
        .iter()
        .filter_map(|r| find_tool(r.tool).map(|path| (path, r)))
        .collect()
}

pub fn listening(config: &VoiceConfig) -> Listening {
    let model = match find_program(config).and_then(|_| find_model(config)) {
        Ok(model) => model,
        Err(reason) => return Listening::Unavailable(reason),
    };
    let recorders = available_recorders();
    let Some((_, first)) = recorders.first() else {
        return Listening::Unavailable(recorder_hint().into());
    };
    Listening::Ready(format!(
        "whisper.cpp · {} · microphone via {}",
        model
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        first.tool
    ))
}

pub fn dictate(
    timeout: Duration,
    config: &VoiceConfig,
    cancel: &Cancellation,
) -> Result<String, Error> {
    let program = find_program(config).map_err(Error::Failed)?;
    let model = find_model(config).map_err(Error::Failed)?;
    let recorders = available_recorders();
    if recorders.is_empty() {
        return Err(Error::Failed(recorder_hint().into()));
    }
    let limits = Limits::conversational(timeout);
    let mut silent = Vec::new();
    for (path, recorder) in recorders {
        let mut command = Command::new(&path);
        command.args(recorder.args);
        match record(command, limits, cancel)? {
            Capture::Speech(pcm) => return transcribe(&program, &model, &pcm, cancel),
            Capture::Silence => return Ok(String::new()),
            // This recorder could not open the microphone; try the next one.
            Capture::NoAudio => silent.push(recorder.tool),
        }
    }
    Err(Error::Failed(format!(
        "No microphone audio from {}. Check that a microphone is connected and that James is allowed to use it.",
        silent.join(", ")
    )))
}

/// Streams one utterance from a recorder process, then stops the recorder.
fn record(mut command: Command, limits: Limits, cancel: &Cancellation) -> Result<Capture, Error> {
    if cancel.is_cancelled() {
        return Err(Error::Cancelled);
    }
    let Ok(mut child) = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return Ok(Capture::NoAudio);
    };
    let stdout = child.stdout.take().expect("piped stdout");
    let result = listen::capture(stdout, limits, NO_AUDIO_AFTER, cancel);
    // Stopping the recorder also closes the pipe the reader thread waits on.
    let _ = child.kill();
    let _ = child.wait();
    result
}

fn transcribe(
    program: &Path,
    model: &Path,
    pcm: &[i16],
    cancel: &Cancellation,
) -> Result<String, Error> {
    let threads = std::thread::available_parallelism()
        .map_or(4, |n| n.get().min(8))
        .to_string();
    let language = listen::language_for(
        model
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default(),
    );
    let audio = listen::wav(pcm);
    let raw = with_private_dir(|dir| {
        let path = dir.join("utterance.wav");
        std::fs::write(&path, &audio)
            .map_err(|e| Error::Failed(format!("Cannot prepare audio for whisper.cpp: {e}")))?;
        let mut command = Command::new(program);
        command
            .arg("-m")
            .arg(model)
            .arg("-f")
            .arg(&path)
            // No timestamps, no progress output: stdout is just the words.
            .args(["-nt", "-np", "-l", language, "-t", &threads]);
        supervise(command, &[], TRANSCRIBE_LIMIT, cancel)
    })
    .map_err(|e| match e {
        Error::Failed(detail) if detail.is_empty() => {
            Error::Failed("whisper.cpp could not transcribe the recording".into())
        }
        Error::Failed(detail) => Error::Failed(format!("whisper.cpp: {detail}")),
        other => other,
    })?;
    Ok(listen::clean_transcript(&String::from_utf8_lossy(&raw)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "james-whisper-test-{}-{name}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn script(dir: &Path, body: &str) -> PathBuf {
        let path = dir.join("fake-whisper");
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    fn speech() -> Vec<u8> {
        let mut pcm = vec![0_i16; 16 * 400];
        pcm.extend((0..16 * 900).map(|i| (3000.0 * (i as f32 * 0.0864).sin()) as i16));
        pcm.extend(vec![0_i16; 16 * 1500]);
        pcm.iter().flat_map(|s| s.to_le_bytes()).collect()
    }

    #[test]
    fn recorders_are_described_for_this_platform() {
        assert!(!RECORDERS.is_empty());
        for recorder in RECORDERS {
            assert!(!recorder.tool.contains('/'));
            assert!(
                recorder
                    .args
                    .iter()
                    .all(|a| !a.contains(';') && !a.contains('|'))
            );
        }
    }

    #[test]
    fn a_recorder_streaming_speech_yields_an_utterance() {
        let dir = scratch("record");
        let audio = dir.join("speech.raw");
        std::fs::write(&audio, speech()).unwrap();
        let mut command = Command::new(find_tool("cat").expect("cat"));
        command.arg(&audio);
        let result = record(
            command,
            Limits::conversational(Duration::from_secs(3)),
            &Cancellation::default(),
        )
        .unwrap();
        assert!(matches!(result, Capture::Speech(_)), "{result:?}");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_recorder_that_fails_reports_no_audio() {
        let command = Command::new(find_tool("false").expect("false"));
        let result = record(
            command,
            Limits::conversational(Duration::from_secs(3)),
            &Cancellation::default(),
        )
        .unwrap();
        assert!(matches!(result, Capture::NoAudio), "{result:?}");
        let missing = Command::new("/definitely/not/a/recorder");
        assert!(matches!(
            record(
                missing,
                Limits::conversational(Duration::from_secs(3)),
                &Cancellation::default()
            )
            .unwrap(),
            Capture::NoAudio
        ));
    }

    #[test]
    fn transcription_uses_a_private_wav_that_is_deleted() {
        let dir = scratch("transcribe");
        // Arguments: -m MODEL -f WAV ... — check the WAV, report its path.
        let fake = script(
            &dir,
            r#"[ "$1" = "-m" ] && [ "$3" = "-f" ] || exit 2
head -c 4 "$4" | grep -q RIFF || exit 3
printf ' [BLANK_AUDIO]\n %s\n' "$4""#,
        );
        let model = dir.join("ggml-base.en.bin");
        std::fs::write(&model, b"model").unwrap();
        let out = transcribe(&fake, &model, &[0, 1, 2, 3], &Cancellation::default()).unwrap();
        assert!(out.ends_with("utterance.wav"), "{out}");
        assert!(!Path::new(&out).exists(), "temporary audio must be deleted");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn whisper_failures_are_explained() {
        let dir = scratch("fail");
        let fake = script(&dir, "echo 'failed to load model' >&2; exit 1");
        let model = dir.join("ggml-tiny.bin");
        std::fs::write(&model, b"model").unwrap();
        let err = transcribe(&fake, &model, &[0; 32], &Cancellation::default()).unwrap_err();
        assert_eq!(err.to_string(), "whisper.cpp: failed to load model");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn explicit_settings_win_and_are_validated() {
        let dir = scratch("settings");
        let fake = script(&dir, "exit 0");
        let model = dir.join("my-model.bin");
        std::fs::write(&model, b"model").unwrap();
        let config = VoiceConfig {
            whisper_program: Some(fake.clone()),
            whisper_model: Some(model.clone()),
        };
        assert_eq!(find_program(&config).unwrap(), fake);
        assert_eq!(find_model(&config).unwrap(), model);
        let relative = VoiceConfig {
            whisper_program: Some(PathBuf::from("whisper-cli")),
            whisper_model: Some(PathBuf::from("model.bin")),
        };
        assert!(find_program(&relative).is_err());
        assert!(find_model(&relative).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
