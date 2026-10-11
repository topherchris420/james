//! Platform-neutral listening pieces: an energy-based endpointer that decides
//! when speech starts and ends, a streaming capture loop, WAV encoding for
//! whisper.cpp, and transcript cleanup. All timing is measured in audio
//! samples, so behaviour is deterministic and testable without a microphone.
use crate::{Cancellation, Error};
use std::collections::VecDeque;
use std::io::Read;
use std::sync::mpsc;
use std::time::{Duration, Instant};

pub const SAMPLE_RATE: u32 = 16_000;
const FRAME: usize = 480; // 30 ms
const NOISE_WINDOW: usize = 50; // noise floor = quietest frame in the last 1.5 s
const PREROLL_FRAMES: usize = 10; // keep 300 ms before speech
// 120 ms of voice starts an utterance. A transient of d ms touches at most
// ceil(d / 30) + 1 frames, so clicks and taps of 60 ms or less never start one,
// however they line up with frame boundaries.
const START_FRAMES: usize = 4;
const MIN_THRESHOLD: f32 = 120.0;
const NOISE_FACTOR: f32 = 3.0;

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// Give up if no speech starts within this much audio.
    pub initial_silence: Duration,
    /// End the utterance after this much quiet following speech.
    pub trailing_silence: Duration,
    /// Hard cap on one utterance.
    pub max_utterance: Duration,
}

impl Limits {
    pub fn conversational(initial_silence: Duration) -> Self {
        Self {
            initial_silence,
            trailing_silence: Duration::from_millis(1200),
            max_utterance: Duration::from_secs(30),
        }
    }
}

fn frames(duration: Duration) -> usize {
    (duration.as_millis() as usize * SAMPLE_RATE as usize / 1000).div_ceil(FRAME)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Waiting,
    Speaking,
    Finished,
    NoSpeech,
}

pub struct Endpointer {
    initial_frames: usize,
    trailing_frames: usize,
    max_frames: usize,
    seen: usize,
    recent: VecDeque<f32>,
    threshold: f32,
    voiced_run: usize,
    silent_run: usize,
    speech_frames: usize,
    preroll: VecDeque<Vec<i16>>,
    audio: Vec<i16>,
    started: bool,
    status: Status,
}

impl Endpointer {
    pub fn new(limits: Limits) -> Self {
        Self {
            initial_frames: frames(limits.initial_silence).max(1),
            trailing_frames: frames(limits.trailing_silence).max(1),
            max_frames: frames(limits.max_utterance).max(1),
            seen: 0,
            recent: VecDeque::with_capacity(NOISE_WINDOW + 1),
            threshold: MIN_THRESHOLD,
            voiced_run: 0,
            silent_run: 0,
            speech_frames: 0,
            preroll: VecDeque::with_capacity(PREROLL_FRAMES + 1),
            audio: Vec::new(),
            started: false,
            status: Status::Waiting,
        }
    }

    /// Feeds exactly one 30 ms frame. Calls after a final status are ignored.
    pub fn push_frame(&mut self, frame: &[i16]) -> Status {
        if matches!(self.status, Status::Finished | Status::NoSpeech) {
            return self.status;
        }
        self.seen += 1;
        let energy = rms(frame);
        if !self.started {
            // Gaps between words are quiet, so the quietest recent frame tracks
            // the room's noise even when the user starts talking immediately.
            let floor = self.recent.iter().copied().fold(f32::MAX, f32::min);
            self.threshold = if floor == f32::MAX {
                MIN_THRESHOLD
            } else {
                (floor * NOISE_FACTOR).max(MIN_THRESHOLD)
            };
            let voiced = energy > self.threshold;
            self.recent.push_back(energy);
            if self.recent.len() > NOISE_WINDOW {
                self.recent.pop_front();
            }
            self.preroll.push_back(frame.to_vec());
            if self.preroll.len() > PREROLL_FRAMES {
                self.preroll.pop_front();
            }
            self.voiced_run = if voiced { self.voiced_run + 1 } else { 0 };
            if self.voiced_run >= START_FRAMES {
                self.started = true;
                self.status = Status::Speaking;
                self.speech_frames = self.voiced_run;
                for earlier in self.preroll.drain(..) {
                    self.audio.extend_from_slice(&earlier);
                }
            } else if self.seen >= self.initial_frames {
                self.status = Status::NoSpeech;
            }
            return self.status;
        }
        // The threshold is frozen at speech start so continuous talking
        // cannot raise its own noise estimate and cut the user off.
        let voiced = energy > self.threshold;
        self.audio.extend_from_slice(frame);
        self.speech_frames += 1;
        self.silent_run = if voiced { 0 } else { self.silent_run + 1 };
        if self.silent_run >= self.trailing_frames || self.speech_frames >= self.max_frames {
            self.status = Status::Finished;
        }
        self.status
    }

    /// Ends input early (the recorder stopped). Speech heard so far still counts.
    pub fn finish(&mut self) -> Status {
        if !matches!(self.status, Status::Finished | Status::NoSpeech) {
            self.status = if self.started {
                Status::Finished
            } else {
                Status::NoSpeech
            };
        }
        self.status
    }

    pub fn into_audio(self) -> Option<Vec<i16>> {
        (self.status == Status::Finished).then_some(self.audio)
    }
}

fn rms(frame: &[i16]) -> f32 {
    if frame.is_empty() {
        return 0.0;
    }
    let sum: f64 = frame.iter().map(|&s| f64::from(s) * f64::from(s)).sum();
    (sum / frame.len() as f64).sqrt() as f32
}

/// What one capture produced.
#[derive(Debug)]
pub enum Capture {
    /// Speech followed by a pause (or the recorder ending).
    Speech(Vec<i16>),
    /// No speech before the initial-silence limit.
    Silence,
    /// The source ended before delivering any audio.
    NoAudio,
}

/// Reads 16 kHz mono signed 16-bit little-endian PCM from `source` until the
/// endpointer decides, the source ends, the wall-clock guard expires or the
/// user cancels. The caller owns and stops the process behind `source`.
pub fn capture(
    source: impl Read + Send + 'static,
    limits: Limits,
    no_audio_after: Duration,
    cancel: &Cancellation,
) -> Result<Capture, Error> {
    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    std::thread::spawn(move || {
        let mut source = source;
        let mut buffer = vec![0_u8; 3200];
        loop {
            match source.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if tx.send(buffer[..n].to_vec()).is_err() {
                        break;
                    }
                }
            }
        }
    });
    let mut endpointer = Endpointer::new(limits);
    let mut pending: Vec<u8> = Vec::with_capacity(FRAME * 4);
    let mut received = 0_usize;
    let start = Instant::now();
    // Audio time drives decisions; this only catches a stalled recorder.
    let guard = limits.initial_silence + limits.max_utterance + Duration::from_secs(5);
    let mut frame = Vec::with_capacity(FRAME);
    loop {
        if cancel.is_cancelled() {
            return Err(Error::Cancelled);
        }
        let status = match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(bytes) => {
                received += bytes.len();
                pending.extend_from_slice(&bytes);
                let mut status = Status::Waiting;
                // `whole` is a multiple of the frame size, so every chunk is a
                // full frame of complete little-endian samples.
                let whole = pending.len() / (FRAME * 2) * (FRAME * 2);
                for chunk in pending[..whole].chunks(FRAME * 2) {
                    frame.clear();
                    frame.extend(chunk.chunks(2).map(|b| i16::from_le_bytes([b[0], b[1]])));
                    status = endpointer.push_frame(&frame);
                    if matches!(status, Status::Finished | Status::NoSpeech) {
                        break;
                    }
                }
                pending.drain(..whole);
                status
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if received == 0 && start.elapsed() >= no_audio_after {
                    return Ok(Capture::NoAudio);
                }
                if start.elapsed() >= guard {
                    endpointer.finish()
                } else {
                    Status::Waiting
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                if received == 0 {
                    return Ok(Capture::NoAudio);
                }
                endpointer.finish()
            }
        };
        match status {
            Status::Finished => {
                return Ok(endpointer
                    .into_audio()
                    .map_or(Capture::Silence, Capture::Speech));
            }
            Status::NoSpeech => return Ok(Capture::Silence),
            Status::Waiting | Status::Speaking => {}
        }
    }
}

/// Encodes 16 kHz mono PCM as a WAV file, the input format whisper.cpp expects.
pub fn wav(pcm: &[i16]) -> Vec<u8> {
    let data_len = (pcm.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + pcm.len() * 2);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16_u32.to_le_bytes()); // fmt chunk size
    out.extend_from_slice(&1_u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1_u16.to_le_bytes()); // mono
    out.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    out.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes()); // byte rate
    out.extend_from_slice(&2_u16.to_le_bytes()); // block align
    out.extend_from_slice(&16_u16.to_le_bytes()); // bits per sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for sample in pcm {
        out.extend_from_slice(&sample.to_le_bytes());
    }
    out
}

/// Joins whisper.cpp's output lines and drops non-speech annotations such as
/// `[BLANK_AUDIO]`, `[Music]` or `(wind blowing)`.
pub fn clean_transcript(raw: &str) -> String {
    let mut out = String::new();
    let mut depth_square = 0_u32;
    let mut depth_round = 0_u32;
    for c in raw.chars() {
        match c {
            '[' => depth_square += 1,
            ']' if depth_square > 0 => depth_square -= 1,
            '(' => depth_round += 1,
            ')' if depth_round > 0 => depth_round -= 1,
            _ if depth_square == 0 && depth_round == 0 => {
                out.push(if c.is_control() { ' ' } else { c })
            }
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// English-only models (`*.en.bin`) must be told English; others auto-detect.
pub fn language_for(model_file_name: &str) -> &'static str {
    if model_file_name.to_ascii_lowercase().contains(".en.") {
        "en"
    } else {
        "auto"
    }
}

/// Model files in preference order: good accuracy at conversational speed first.
pub const PREFERRED_MODELS: &[&str] = &[
    "ggml-base.en.bin",
    "ggml-base.bin",
    "ggml-small.en.bin",
    "ggml-small.bin",
    "ggml-base.en-q5_1.bin",
    "ggml-base-q5_1.bin",
    "ggml-tiny.en.bin",
    "ggml-tiny.bin",
    "ggml-medium.en.bin",
    "ggml-medium.bin",
    "ggml-large-v3-turbo.bin",
];

/// Picks a model from file names found in a folder.
pub fn choose_model<'a>(names: &[&'a str]) -> Option<&'a str> {
    PREFERRED_MODELS
        .iter()
        .find_map(|preferred| names.iter().copied().find(|n| n == preferred))
        .or_else(|| {
            let mut others: Vec<&str> = names
                .iter()
                .copied()
                .filter(|n| n.starts_with("ggml-") && n.ends_with(".bin"))
                .collect();
            others.sort_unstable();
            others.first().copied()
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn silence(ms: usize) -> Vec<i16> {
        vec![0; ms * 16]
    }
    fn noise(ms: usize, amplitude: i16, seed: &mut u32) -> Vec<i16> {
        (0..ms * 16)
            .map(|_| {
                *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (*seed >> 16) as i16 % amplitude.max(1)
            })
            .collect()
    }
    fn tone(ms: usize, amplitude: f32) -> Vec<i16> {
        (0..ms * 16)
            .map(|i| {
                (amplitude * (i as f32 * 2.0 * std::f32::consts::PI * 220.0 / 16_000.0).sin())
                    as i16
            })
            .collect()
    }
    fn bytes(samples: &[i16]) -> Vec<u8> {
        samples.iter().flat_map(|s| s.to_le_bytes()).collect()
    }
    fn limits() -> Limits {
        Limits::conversational(Duration::from_secs(2))
    }

    #[test]
    fn speech_then_pause_ends_the_utterance_with_preroll() {
        let mut audio = silence(500);
        audio.extend(tone(1000, 3000.0));
        audio.extend(silence(2000));
        let result = capture(
            Cursor::new(bytes(&audio)),
            limits(),
            Duration::from_secs(5),
            &Cancellation::default(),
        )
        .unwrap();
        let Capture::Speech(pcm) = result else {
            panic!("expected speech, got {result:?}")
        };
        // ~300 ms preroll + 1 s speech + 1.2 s trailing silence, give or take a frame.
        let ms = pcm.len() / 16;
        assert!((2300..=2650).contains(&ms), "{ms} ms");
    }

    #[test]
    fn quiet_room_noise_is_not_speech() {
        let mut seed = 7;
        let audio = noise(3000, 60, &mut seed);
        let result = capture(
            Cursor::new(bytes(&audio)),
            limits(),
            Duration::from_secs(5),
            &Cancellation::default(),
        )
        .unwrap();
        assert!(matches!(result, Capture::Silence), "{result:?}");
    }

    #[test]
    fn speech_over_steady_noise_is_detected() {
        let mut seed = 11;
        let mut audio = noise(600, 300, &mut seed);
        let speech: Vec<i16> = tone(800, 4000.0)
            .into_iter()
            .zip(noise(800, 300, &mut seed))
            .map(|(a, b)| a.saturating_add(b))
            .collect();
        audio.extend(speech);
        audio.extend(noise(1500, 300, &mut seed));
        let result = capture(
            Cursor::new(bytes(&audio)),
            limits(),
            Duration::from_secs(5),
            &Cancellation::default(),
        )
        .unwrap();
        assert!(matches!(result, Capture::Speech(_)), "{result:?}");
    }

    #[test]
    fn talking_immediately_still_calibrates_from_gaps() {
        let mut audio = tone(300, 3000.0);
        audio.extend(silence(150));
        audio.extend(tone(600, 3000.0));
        audio.extend(silence(1500));
        let result = capture(
            Cursor::new(bytes(&audio)),
            limits(),
            Duration::from_secs(5),
            &Cancellation::default(),
        )
        .unwrap();
        assert!(matches!(result, Capture::Speech(_)), "{result:?}");
    }

    #[test]
    fn clicks_of_sixty_ms_or_less_never_start_speech() {
        // Clicks at mid-frame offsets straddle three frames each.
        let mut audio = silence(500);
        audio.extend(tone(60, 8000.0));
        audio.extend(silence(310));
        audio.extend(tone(60, 8000.0));
        audio.extend(silence(2000));
        let result = capture(
            Cursor::new(bytes(&audio)),
            limits(),
            Duration::from_secs(5),
            &Cancellation::default(),
        )
        .unwrap();
        assert!(matches!(result, Capture::Silence), "{result:?}");
    }

    #[test]
    fn long_speech_is_capped() {
        let mut audio = silence(400);
        audio.extend(tone(5000, 3000.0));
        let limits = Limits {
            max_utterance: Duration::from_secs(2),
            ..limits()
        };
        let Capture::Speech(pcm) = capture(
            Cursor::new(bytes(&audio)),
            limits,
            Duration::from_secs(5),
            &Cancellation::default(),
        )
        .unwrap() else {
            panic!("expected speech")
        };
        assert!(pcm.len() / 16 <= 2400, "{} ms", pcm.len() / 16);
    }

    #[test]
    fn recorder_ending_mid_speech_keeps_what_was_said() {
        let mut audio = silence(400);
        audio.extend(tone(700, 3000.0));
        let result = capture(
            Cursor::new(bytes(&audio)),
            limits(),
            Duration::from_secs(5),
            &Cancellation::default(),
        )
        .unwrap();
        assert!(matches!(result, Capture::Speech(_)), "{result:?}");
    }

    #[test]
    fn empty_source_reports_no_audio_and_cancel_wins() {
        let result = capture(
            Cursor::new(Vec::new()),
            limits(),
            Duration::from_secs(5),
            &Cancellation::default(),
        )
        .unwrap();
        assert!(matches!(result, Capture::NoAudio), "{result:?}");
        let cancel = Cancellation::default();
        cancel.cancel();
        assert!(matches!(
            capture(
                Cursor::new(bytes(&silence(100))),
                limits(),
                Duration::from_secs(5),
                &cancel
            ),
            Err(Error::Cancelled)
        ));
    }

    #[test]
    fn odd_byte_boundaries_are_reassembled() {
        struct Dribble(Vec<u8>, usize);
        impl Read for Dribble {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                let n = 333.min(self.0.len() - self.1).min(buf.len());
                buf[..n].copy_from_slice(&self.0[self.1..self.1 + n]);
                self.1 += n;
                Ok(n)
            }
        }
        let mut audio = silence(400);
        audio.extend(tone(800, 3000.0));
        audio.extend(silence(1500));
        let result = capture(
            Dribble(bytes(&audio), 0),
            limits(),
            Duration::from_secs(5),
            &Cancellation::default(),
        )
        .unwrap();
        assert!(matches!(result, Capture::Speech(_)), "{result:?}");
    }

    #[test]
    fn wav_header_describes_sixteen_khz_mono_pcm() {
        let file = wav(&[1, -1, 256]);
        assert_eq!(&file[..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(file[4..8].try_into().unwrap()), 36 + 6);
        assert_eq!(&file[8..16], b"WAVEfmt ");
        assert_eq!(u32::from_le_bytes(file[24..28].try_into().unwrap()), 16_000);
        assert_eq!(u16::from_le_bytes(file[34..36].try_into().unwrap()), 16);
        assert_eq!(&file[36..40], b"data");
        assert_eq!(file.len(), 44 + 6);
        assert_eq!(&file[44..], &[1, 0, 255, 255, 0, 1]);
    }

    #[test]
    fn transcripts_drop_annotations_and_join_lines() {
        assert_eq!(clean_transcript("[BLANK_AUDIO]\n"), "");
        assert_eq!(
            clean_transcript(" What's on my screen?\n (wind blowing) Next step [Music]\n"),
            "What's on my screen? Next step"
        );
        assert_eq!(
            clean_transcript("unbalanced ] and ) stay"),
            "unbalanced ] and ) stay"
        );
    }

    #[test]
    fn english_only_models_get_english() {
        assert_eq!(language_for("ggml-base.en.bin"), "en");
        assert_eq!(language_for("GGML-TINY.EN.BIN"), "en");
        assert_eq!(language_for("ggml-small.bin"), "auto");
        assert_eq!(language_for("ggml-large-v3-turbo.bin"), "auto");
    }

    #[test]
    fn models_are_chosen_by_preference_then_name() {
        assert_eq!(
            choose_model(&["ggml-tiny.bin", "ggml-base.en.bin"]),
            Some("ggml-base.en.bin")
        );
        assert_eq!(
            choose_model(&["notes.txt", "ggml-zz.bin", "ggml-aa.bin"]),
            Some("ggml-aa.bin")
        );
        assert_eq!(choose_model(&["model.gguf"]), None);
    }
}
