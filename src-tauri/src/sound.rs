//! The sounds at start, end and cancel: built in (the Echo Dot's ping, pong and break) or a WAV file.

use std::io::Cursor;

use mundwerk_core::config::Sounds;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};

const BUILTIN_PREFIX: &str = "builtin:";
const BUILTIN: &[(&str, &[u8])] = &[
    ("ping", include_bytes!("../../assets/sounds/ping.wav")),
    ("pong", include_bytes!("../../assets/sounds/pong.wav")),
    ("break", include_bytes!("../../assets/sounds/break.wav")),
];

#[derive(Clone, Copy)]
pub enum Cue {
    Start,
    Stop,
    Cancel,
}

pub struct Player {
    // The output stays open for the life of the player; dropping it silences every sound.
    _stream: OutputStream,
    handle: OutputStreamHandle,
}

impl Player {
    pub fn new() -> Result<Self, String> {
        let (stream, handle) = OutputStream::try_default().map_err(|error| error.to_string())?;
        Ok(Self { _stream: stream, handle })
    }

    /// Play the sound for `cue` without waiting for it to end.
    pub fn play(&self, sounds: &Sounds, cue: Cue) -> Result<(), String> {
        if !sounds.enabled {
            return Ok(());
        }
        let source = match cue {
            Cue::Start => &sounds.start,
            Cue::Stop => &sounds.stop,
            Cue::Cancel => &sounds.cancel,
        };
        let bytes = load(source)?;
        let decoder = Decoder::new(Cursor::new(bytes)).map_err(|error| format!("sound {source:?}: {error}"))?;
        let sink = Sink::try_new(&self.handle).map_err(|error| error.to_string())?;
        sink.set_volume(sounds.volume);
        sink.append(decoder);
        sink.detach();
        Ok(())
    }
}

fn load(source: &str) -> Result<Vec<u8>, String> {
    match source.strip_prefix(BUILTIN_PREFIX) {
        Some(name) => BUILTIN
            .iter()
            .find(|(builtin, _)| *builtin == name)
            .map(|(_, bytes)| bytes.to_vec())
            .ok_or_else(|| format!("no built-in sound {name:?} (ping, pong, break)")),
        None => std::fs::read(source).map_err(|error| format!("sound {source:?}: {error}")),
    }
}
