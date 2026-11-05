//! Mundwerk's logic without a user interface, testable on any machine: the settings, the triggers as text,
//! when a dictation starts and ends, the replacement list, the space between dictations, the call to the STT-Suite,
//! the history, the WAV file.

pub mod config;
pub mod controller;
pub mod history;
pub mod replace;
pub mod spacing;
pub mod stt;
pub mod trigger;
pub mod wav;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("settings: {0}")]
    Config(String),
    #[error("file: {0}")]
    Io(#[from] std::io::Error),
    #[error("recording: {0}")]
    Audio(String),
    #[error("no STT-Suite reachable: {}", .0.join("; "))]
    Unreachable(Vec<String>),
    #[error("STT-Suite refused ({status} {code}): {detail}")]
    Refused { status: u16, code: String, detail: String },
    #[error("STT-Suite: {0}")]
    Server(String),
}
