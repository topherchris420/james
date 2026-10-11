//! Fallback for platforms without desktop helpers.
use crate::{Action, Cancellation, Error, Listening, Screenshot, VoiceConfig};
use std::{path::PathBuf, time::Duration};

const MESSAGE: &str = "This capability is not available on this platform.";
pub const USES_WHISPER: bool = false;
pub const CAN_CONTROL_INPUT: bool = false;
pub const VOICE_SUPPORT: &str = "Voice is not available on this platform.";

pub fn listening(_: &VoiceConfig) -> Listening {
    Listening::Unavailable(MESSAGE.into())
}
pub fn whisper_models_dir() -> Option<PathBuf> {
    None
}
pub fn capture_screen(_: &Cancellation) -> Result<Screenshot, Error> {
    Err(Error::Unsupported(MESSAGE))
}
pub fn dictate(_: Duration, _: &VoiceConfig, _: &Cancellation) -> Result<String, Error> {
    Err(Error::Unsupported(MESSAGE))
}
pub fn speak(_: &str, _: &Cancellation) -> Result<(), Error> {
    Err(Error::Unsupported(MESSAGE))
}
pub fn execute(_: &Action, _: u32, _: u32, _: &Cancellation) -> Result<(), Error> {
    Err(Error::Unsupported(MESSAGE))
}
