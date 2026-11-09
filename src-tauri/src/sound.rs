//! The sounds at start, end and cancel: built in (ping, pong, break) or a WAV file. The settings
//! window plays one to listen to it before it is saved.

use std::io::Cursor;
use std::sync::mpsc;
use std::thread::JoinHandle;

use mundwerk_core::config::{fraction, Sounds};
use rodio::{Decoder, OutputStream, Sink};

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

/// Play the sound for `cue`; `to_end`: return only when it has ended (the start sound, before the microphone
/// opens: an open microphone turns a headset to its telephone mode, and the sound would come quiet, cut or with
/// an echo, and into the recording; Agent-Orc does it the same way).
pub fn play(sounds: &Sounds, cue: Cue, to_end: bool) -> Result<(), String> {
    if !sounds.enabled {
        return Ok(());
    }
    let source = match cue {
        Cue::Start => &sounds.start,
        Cue::Stop => &sounds.stop,
        Cue::Cancel => &sounds.cancel,
    };
    let playing = start(source, sounds.volume_percent)?;
    if to_end {
        // A thread that panicked has ended its sound as well.
        let _ = playing.join();
    }
    Ok(())
}

/// Play `source` once, to listen to it (the settings window).
pub fn preview(source: &str, volume_percent: u8) -> Result<(), String> {
    start(source, volume_percent).map(drop)
}

/// Play `source` on the output that is the default now, on a thread of its own (an output cannot move between
/// threads); returns once it plays, with the thread, which ends with the sound. Opened anew for every sound: an
/// output kept open since the start went silent with its device (headset off, screen asleep), and waiting for
/// the end of a sound on it never returned, so no dictation started any more.
fn start(source: &str, volume_percent: u8) -> Result<JoinHandle<()>, String> {
    let decoded = decode(source)?;
    let (opened, outcome) = mpsc::channel();
    let thread = std::thread::Builder::new()
        .name("sound".into())
        .spawn(move || {
            let output = OutputStream::try_default().map_err(|error| error.to_string()).and_then(|(stream, handle)| {
                let sink = Sink::try_new(&handle).map_err(|error| error.to_string())?;
                sink.set_volume(fraction(volume_percent));
                sink.append(decoded);
                Ok((stream, sink))
            });
            match output {
                // The stream lives until the sound has ended; dropping it would silence the sound.
                Ok((_stream, sink)) => {
                    let _ = opened.send(Ok(()));
                    sink.sleep_until_end();
                }
                Err(error) => {
                    let _ = opened.send(Err(error));
                }
            }
        })
        .map_err(|error| error.to_string())?;
    outcome.recv().map_err(|error| error.to_string())??;
    Ok(thread)
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
