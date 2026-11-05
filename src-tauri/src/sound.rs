//! The sounds at start, end and cancel: built in (ping, pong, break) or a WAV file. The settings
//! window plays one to listen to it before it is saved.

use std::io::Cursor;

use mundwerk_core::config::{fraction, Sounds};
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
        self.start(source, sounds.volume_percent)?.detach();
        Ok(())
    }

    /// The sound playing; it stops when the returned sink is dropped unless it is detached.
    fn start(&self, source: &str, volume_percent: u8) -> Result<Sink, String> {
        let sink = Sink::try_new(&self.handle).map_err(|error| error.to_string())?;
        sink.set_volume(fraction(volume_percent));
        sink.append(decode(source)?);
        Ok(sink)
    }
}

/// Play `source` once, to listen to it (the settings window). A sound that cannot be read is said at once; the
/// sound itself plays on a thread of its own, with an output of its own (an output cannot move between threads).
pub fn preview(source: &str, volume_percent: u8) -> Result<(), String> {
    decode(source)?;
    let source = source.to_owned();
    std::thread::spawn(move || {
        let played = Player::new().and_then(|player| {
            player.start(&source, volume_percent)?.sleep_until_end();
            Ok(())
        });
        if let Err(error) = played {
            log::error!("sound preview: {error}");
        }
    });
    Ok(())
}

fn decode(source: &str) -> Result<Decoder<Cursor<Vec<u8>>>, String> {
    let bytes = match source.strip_prefix(BUILTIN_PREFIX) {
        Some(name) => BUILTIN
            .iter()
            .find(|(builtin, _)| *builtin == name)
            .map(|(_, bytes)| bytes.to_vec())
            .ok_or_else(|| format!("no built-in sound {name:?} (ping, pong, break)"))?,
        None => std::fs::read(source).map_err(|error| format!("sound {source:?}: {error}"))?,
    };
    Decoder::new(Cursor::new(bytes)).map_err(|error| format!("sound {source:?}: {error}"))
}
