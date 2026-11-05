//! Against a running STT-Suite, only when asked for:
//!
//! ```text
//! MUNDWERK_LIVE_URL=http://192.168.0.249:5080 MUNDWERK_LIVE_TOKEN_FILE=~/.config/stt-suite/token \
//! MUNDWERK_LIVE_WAV=~/Projekte/STT-Suite/burn_in_speech.wav cargo test -p mundwerk-core --test live_stt -- --ignored
//! ```

use std::path::PathBuf;

use mundwerk_core::config::{Config, Device, DEFAULT_CONFIG};
use mundwerk_core::stt::SttClient;
use mundwerk_core::Error;

/// A port nobody listens on: the connection is refused at once, as on a cable that was pulled.
const NOBODY: &str = "http://127.0.0.1:9";

fn setting(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("{name} is needed for the live test"))
}

fn expand(path: &str) -> PathBuf {
    match path.strip_prefix("~/") {
        Some(rest) => PathBuf::from(setting("HOME")).join(rest),
        None => PathBuf::from(path),
    }
}

fn client(urls: Vec<String>, token: String) -> SttClient {
    let mut config = Config::parse(DEFAULT_CONFIG).unwrap();
    config.server.urls = urls;
    config.server.token = token;
    config.transcription.device = Device::Cpu;
    SttClient::new(config.server, config.transcription).unwrap()
}

#[test]
#[ignore = "needs a running STT-Suite"]
fn transcribes_through_the_second_server_when_the_first_is_gone() {
    let token = std::fs::read_to_string(expand(&setting("MUNDWERK_LIVE_TOKEN_FILE"))).unwrap().trim().to_owned();
    let wav = std::fs::read(expand(&setting("MUNDWERK_LIVE_WAV"))).unwrap();
    let live = setting("MUNDWERK_LIVE_URL");
    let transcript = client(vec![NOBODY.into(), live.clone()], token).transcribe(&wav).unwrap();
    assert_eq!(transcript.server, live);
    assert_eq!(transcript.device, "cpu");
    assert!(transcript.text.split_whitespace().count() > 20, "{}", transcript.text);
    println!("{} words via {}: {}", transcript.text.split_whitespace().count(), transcript.server, transcript.text);
}

#[test]
#[ignore = "needs a running STT-Suite"]
fn a_wrong_token_is_refused_and_not_passed_on() {
    let wav = std::fs::read(expand(&setting("MUNDWERK_LIVE_WAV"))).unwrap();
    let live = setting("MUNDWERK_LIVE_URL");
    match client(vec![live, NOBODY.into()], "wrong".into()).transcribe(&wav) {
        Err(Error::Refused { status: 401, code, .. }) => assert_eq!(code, "unauthorized"),
        other => panic!("expected 401, got {other:?}"),
    }
}

#[test]
#[ignore = "needs a running STT-Suite"]
fn nobody_there_names_every_server() {
    match client(vec![NOBODY.into(), "http://127.0.0.1:10".into()], String::new()).transcribe(b"") {
        Err(Error::Unreachable(reasons)) => assert_eq!(reasons.len(), 2),
        other => panic!("expected unreachable, got {other:?}"),
    }
}
