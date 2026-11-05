//! The call to the STT-Suite: `POST /transcribe` with the recording as a WAV file. The servers are tried in the
//! order of the settings (e.g. the direct cable first, the home network second). Only a server that cannot be
//! reached is skipped; one that answers with an error has answered, and that answer is what the user sees.

use std::time::Duration;

use reqwest::multipart::{Form, Part};
use reqwest::Client;
use serde::Deserialize;
use tokio::runtime::Runtime;

use crate::config::{Server, Transcription};
use crate::Error;

/// The header the STT-Suite reads its token from (not `Authorization`, which belongs to a reverse proxy).
pub const TOKEN_HEADER: &str = "X-STT-Suite-Token";

/// What the STT-Suite answered, and which server it was.
#[derive(Debug, Clone, PartialEq)]
pub struct Transcript {
    pub text: String,
    pub server: String,
    pub device: String,
    pub engine: String,
}

#[derive(Deserialize)]
struct Answer {
    text: String,
    device: String,
    engine: String,
}

#[derive(Deserialize)]
struct Refusal {
    error: String,
    detail: String,
}

pub struct SttClient {
    // The async client inside a runtime of its own: the blocking client streams a multipart body from another
    // thread and then reports a refused connection and an early answer (401 before the upload is done) as a
    // body error, so neither the fallback nor the refusal would work.
    runtime: Runtime,
    client: Client,
    server: Server,
    transcription: Transcription,
}

impl SttClient {
    /// # Errors
    /// The HTTP client or its runtime cannot be built.
    pub fn new(server: Server, transcription: Transcription) -> Result<Self, Error> {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
        // No overall time limit (the async client sets none): a long dictation takes as long as it takes on the CPU.
        let client = Client::builder()
            .connect_timeout(Duration::from_millis(server.connect_timeout_ms))
            .user_agent(concat!("Mundwerk/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|error| Error::Server(error.to_string()))?;
        Ok(Self { runtime, client, server, transcription })
    }

    /// Transcribe one recording (`wav`: the whole WAV file).
    ///
    /// # Errors
    /// [`Error::Unreachable`] when no server could be reached (each with its reason); [`Error::Refused`] when
    /// one answered with an error; [`Error::Server`] for an answer that is not what the STT-Suite sends.
    pub fn transcribe(&self, wav: &[u8]) -> Result<Transcript, Error> {
        let mut unreachable = Vec::new();
        for url in &self.server.urls {
            match self.runtime.block_on(self.send(url, wav)) {
                Err(Attempt::Unreachable(reason)) => unreachable.push(format!("{url}: {reason}")),
                Err(Attempt::Failed(error)) => return Err(error),
                Ok(transcript) => return Ok(transcript),
            }
        }
        Err(Error::Unreachable(unreachable))
    }

    async fn send(&self, url: &str, wav: &[u8]) -> Result<Transcript, Attempt> {
        let file = Part::bytes(wav.to_vec())
            .file_name("dictation.wav")
            .mime_str("audio/wav")
            .map_err(|error| Attempt::Failed(Error::Server(error.to_string())))?;
        // No engine or model: the STT-Suite uses what it has set for the device.
        let form = Form::new()
            .part("file", file)
            .text("device", self.transcription.device.as_str())
            .text("language", self.transcription.language.clone());
        let mut request = self.client.post(format!("{}/transcribe", url.trim_end_matches('/'))).multipart(form);
        if !self.server.token.is_empty() {
            request = request.header(TOKEN_HEADER, &self.server.token);
        }
        let response = request.send().await.map_err(|error| {
            if error.is_connect() {
                Attempt::Unreachable(error.to_string())
            } else {
                Attempt::Failed(Error::Server(format!("{url}: {error}")))
            }
        })?;
        let status = response.status();
        let body = response.text().await.map_err(|error| Attempt::Failed(Error::Server(format!("{url}: {error}"))))?;
        if !status.is_success() {
            let refusal: Refusal = serde_json::from_str(&body)
                .map_err(|_| Attempt::Failed(Error::Server(format!("{url}: {status} {body}"))))?;
            return Err(Attempt::Failed(Error::Refused {
                status: status.as_u16(),
                code: refusal.error,
                detail: refusal.detail,
            }));
        }
        let answer: Answer = serde_json::from_str(&body)
            .map_err(|error| Attempt::Failed(Error::Server(format!("{url}: {error}: {body}"))))?;
        Ok(Transcript { text: answer.text, server: url.to_owned(), device: answer.device, engine: answer.engine })
    }
}

enum Attempt {
    /// No connection: the next server may take it.
    Unreachable(String),
    /// The server was there; what went wrong is the answer.
    Failed(Error),
}
