//! The run of a dictation. The hook thread asks the controller what a press means; the recorder thread opens and
//! closes the microphone, plays the sounds and watches silence and length; the transcriber thread sends one
//! recording after the other to the STT-Suite, corrects the text with the replacement list and writes it into the
//! field in front. So a new dictation can start while the last one is still being transcribed, in order.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use mundwerk_core::config::{fraction, Config};
use mundwerk_core::controller::{Action, Controller, Event};
use mundwerk_core::history::{Entry, History};
use mundwerk_core::replace::Replacer;
use mundwerk_core::stt::SttClient;
use mundwerk_core::wav;

use crate::audio::{self, Recording};
use crate::insert::Inserter;
use crate::sound::{Cue, Player};
use crate::status::{Reporter, Status};

/// How often the recorder looks at silence and length while it records.
const TICK: Duration = Duration::from_millis(100);

pub struct Shared {
    pub config_path: PathBuf,
    pub config: Mutex<Config>,
    pub controller: Mutex<Controller>,
    pub history: Mutex<History>,
    /// Recordings sent to the STT-Suite and not yet back.
    pending: AtomicUsize,
    recorder: Sender<Action>,
}

/// A lock that a panicked thread left behind still holds valid data here (whole values are swapped in).
pub fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Shared {
    /// What the hook calls for every press: `true` = swallow it.
    pub fn press(&self, event: &Event) -> bool {
        let outcome = lock(&self.controller).handle(event, Instant::now());
        if outcome.action != Action::Nothing {
            log::info!("trigger {event:?}: {:?}", outcome.action);
            // The recorder runs as long as the app: a send cannot fail.
            let _ = self.recorder.send(outcome.action);
        }
        outcome.swallow
    }

    /// What Mundwerk is doing now (the one place that decides it): recording wins over transcribing.
    pub fn status(&self) -> Status {
        if lock(&self.controller).is_recording() {
            Status::Listening
        } else if self.pending.load(Ordering::SeqCst) > 0 {
            Status::Transcribing
        } else {
            Status::Idle
        }
    }

    /// New settings (the settings window saved them): triggers, history size and everything else from now on.
    pub fn apply(&self, config: Config) -> Result<(), String> {
        *lock(&self.controller) = Controller::new(&config).map_err(|error| error.to_string())?;
        lock(&self.history).resize(config.history.size);
        *lock(&self.config) = config;
        Ok(())
    }
}

/// Start the recorder and transcriber threads.
pub fn start(config_path: PathBuf, config: Config, reporter: Reporter) -> Result<Arc<Shared>, String> {
    let (recorder, actions) = mpsc::channel();
    let shared = Arc::new(Shared {
        config_path,
        controller: Mutex::new(Controller::new(&config).map_err(|error| error.to_string())?),
        history: Mutex::new(History::new(config.history.size)),
        config: Mutex::new(config),
        pending: AtomicUsize::new(0),
        recorder,
    });
    let (jobs, queue) = mpsc::channel();
    let for_recorder = Arc::clone(&shared);
    let recorder_reporter = reporter.clone();
    std::thread::Builder::new()
        .name("recorder".into())
        .spawn(move || record(&for_recorder, &actions, &jobs, &recorder_reporter))
        .map_err(|error| error.to_string())?;
    let for_transcriber = Arc::clone(&shared);
    std::thread::Builder::new()
        .name("transcriber".into())
        .spawn(move || transcribe(&for_transcriber, &queue, &reporter))
        .map_err(|error| error.to_string())?;
    Ok(shared)
}

/// A recording on its way to the STT-Suite.
struct Job {
    samples: Vec<f32>,
    sample_rate: u32,
}

fn record(shared: &Shared, actions: &Receiver<Action>, jobs: &Sender<Job>, reporter: &Reporter) {
    // The audio output is opened on this thread and lives with it (it cannot move between threads).
    let player = match Player::new() {
        Ok(player) => Some(player),
        Err(error) => {
            reporter.report(Status::Error(format!("sound output: {error}")));
            None
        }
    };
    let play = |cue| {
        if let Some(player) = &player {
            if let Err(error) = player.play(&lock(&shared.config).sounds, cue) {
                reporter.report(Status::Error(error));
            }
        }
    };
    let mut current: Option<Recording> = None;
    loop {
        let action = match actions.recv_timeout(TICK) {
            Ok(action) => action,
            Err(RecvTimeoutError::Timeout) => match &current {
                Some(recording) if limit_reached(&lock(&shared.config), recording) => lock(&shared.controller).finish(),
                _ => continue,
            },
            Err(RecvTimeoutError::Disconnected) => return,
        };
        match action {
            Action::Nothing => {}
            Action::Start => {
                let (microphone, silence_level) = {
                    let config = lock(&shared.config);
                    (config.recording.microphone.clone(), fraction(config.recording.silence_level_percent))
                };
                match audio::start(&microphone, silence_level) {
                    Ok(recording) => {
                        play(Cue::Start);
                        current = Some(recording);
                        reporter.report(Status::Listening);
                    }
                    Err(error) => {
                        // The controller believes it records: put it back, or the next press would "stop".
                        lock(&shared.controller).finish();
                        reporter.report(Status::Error(format!("microphone: {error}")));
                    }
                }
            }
            Action::Stop => {
                if let Some(recording) = current.take() {
                    let (samples, sample_rate) = recording.finish();
                    play(Cue::Stop);
                    shared.pending.fetch_add(1, Ordering::SeqCst);
                    reporter.report(Status::Transcribing);
                    // The transcriber runs as long as the app: a send cannot fail.
                    let _ = jobs.send(Job { samples, sample_rate });
                }
            }
            Action::Discard => {
                current.take();
                reporter.report(shared.status());
            }
            Action::Cancel => {
                current.take();
                play(Cue::Cancel);
                reporter.report(shared.status());
            }
        }
    }
}

fn limit_reached(config: &Config, recording: &Recording) -> bool {
    let silence = config.recording.stop_after_silence_s;
    let maximum = config.recording.max_duration_s;
    (silence > 0 && recording.quiet_for() >= Duration::from_secs(silence))
        || (maximum > 0 && recording.elapsed() >= Duration::from_secs(maximum))
}

fn transcribe(shared: &Shared, queue: &Receiver<Job>, reporter: &Reporter) {
    let mut inserter = match Inserter::new() {
        Ok(inserter) => Some(inserter),
        Err(error) => {
            reporter.report(Status::Error(format!("typing: {error}")));
            None
        }
    };
    for job in queue {
        let result = one(shared, &job, inserter.as_mut());
        shared.pending.fetch_sub(1, Ordering::SeqCst);
        match result {
            Ok(()) => reporter.report(shared.status()),
            Err(error) => reporter.report(Status::Error(error)),
        }
    }
}

/// One recording: to the STT-Suite, through the replacement list, into the field, into the history.
fn one(shared: &Shared, job: &Job, inserter: Option<&mut Inserter>) -> Result<(), String> {
    let config = lock(&shared.config).clone();
    let wav = wav::encode(&job.samples, job.sample_rate).map_err(|error| error.to_string())?;
    let client =
        SttClient::new(config.server.clone(), config.transcription.clone()).map_err(|error| error.to_string())?;
    let transcript = client.transcribe(&wav).map_err(|error| error.to_string())?;
    log::info!(
        "transcribed by {} ({}, {}): {} characters",
        transcript.server,
        transcript.device,
        transcript.engine,
        transcript.text.len()
    );
    let text = Replacer::new(&config.replacements).map_err(|error| error.to_string())?.apply(transcript.text.trim());
    if text.is_empty() {
        return Ok(());
    }
    lock(&shared.history).add(Entry { when: chrono::Local::now().format("%H:%M").to_string(), text: text.clone() });
    inserter.ok_or("typing is not available")?.insert(&config.insert, &text)
}
