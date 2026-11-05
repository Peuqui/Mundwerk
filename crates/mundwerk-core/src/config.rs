//! The settings. Their defaults live in one place, `default_config.toml`; every field must be in the file
//! (no silent defaults), and a field Mundwerk does not know is an error, so a typo does not go unnoticed.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::replace::Replacer;
use crate::trigger::Trigger;
use crate::Error;

/// The settings as Mundwerk writes them on the first start.
pub const DEFAULT_CONFIG: &str = include_str!("../default_config.toml");

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub server: Server,
    pub transcription: Transcription,
    pub triggers: Vec<TriggerSetting>,
    pub recording: Recording,
    pub sounds: Sounds,
    pub insert: Insert,
    pub replacements: Vec<Replacement>,
    pub overlay: Overlay,
    pub history: History,
    pub system: System,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Server {
    pub urls: Vec<String>,
    pub token: String,
    pub connect_timeout_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transcription {
    pub device: String,
    pub language: String,
    pub engine: String,
    pub model: String,
    pub quality: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerMode {
    Hold,
    Toggle,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TriggerSetting {
    pub input: String,
    pub mode: TriggerMode,
    pub pass_through: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recording {
    pub microphone: String,
    pub min_press_ms: u64,
    pub cancel_key: String,
    pub stop_after_silence_s: u64,
    pub max_duration_s: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sounds {
    pub enabled: bool,
    pub start: String,
    pub stop: String,
    pub cancel: String,
    pub volume: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InsertMethod {
    Type,
    Clipboard,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Insert {
    pub method: InsertMethod,
    pub space_between_dictations: bool,
    pub press_enter: bool,
    // An empty list is written as nothing at all; the field may then be missing from the file.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub per_program: Vec<ProgramMethod>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProgramMethod {
    pub process: String,
    pub method: InsertMethod,
}

impl Insert {
    /// The method for the program that has the focus (by process name, case ignored).
    #[must_use]
    pub fn method_for(&self, process: &str) -> InsertMethod {
        self.per_program
            .iter()
            .find(|rule| rule.process.eq_ignore_ascii_case(process))
            .map_or(self.method, |rule| rule.method)
    }
}

/// One rule of the replacement list: either `variants` (whole words, case ignored) or `pattern` (a regular
/// expression), never both.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Replacement {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variants: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    pub replacement: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlayMode {
    Floating,
    TrayOnly,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Overlay {
    pub mode: OverlayMode,
    /// `[x, y]` in screen coordinates, or empty for the default place.
    pub position: Vec<i32>,
    pub opacity: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct History {
    pub size: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct System {
    pub autostart: bool,
}

impl Config {
    /// Read and check the settings in `text`.
    ///
    /// # Errors
    /// The text is no valid settings file, or a value in it cannot work (see [`Config::validate`]).
    pub fn parse(text: &str) -> Result<Self, Error> {
        let config: Self = toml::from_str(text).map_err(|error| Error::Config(error.to_string()))?;
        config.validate()?;
        Ok(config)
    }

    /// The settings from `path`; on the first start (no file) the defaults are written there first.
    ///
    /// # Errors
    /// The file cannot be read or written, or [`Config::parse`] refuses it.
    pub fn load_or_create(path: &Path) -> Result<Self, Error> {
        if !path.exists() {
            if let Some(folder) = path.parent() {
                std::fs::create_dir_all(folder)?;
            }
            std::fs::write(path, DEFAULT_CONFIG)?;
        }
        Self::parse(&std::fs::read_to_string(path)?)
    }

    /// Write the settings to `path` (the settings window saves this way).
    ///
    /// # Errors
    /// The settings cannot work, or the file cannot be written.
    pub fn save(&self, path: &Path) -> Result<(), Error> {
        self.validate()?;
        let text = toml::to_string_pretty(self).map_err(|error| Error::Config(error.to_string()))?;
        std::fs::write(path, text)?;
        Ok(())
    }

    /// What the file cannot say through its types: values that would make Mundwerk fail later.
    ///
    /// # Errors
    /// The first value that cannot work, named.
    pub fn validate(&self) -> Result<(), Error> {
        if self.server.urls.is_empty() {
            return Err(Error::Config("server.urls: at least one STT-Suite is needed".into()));
        }
        for setting in &self.triggers {
            Trigger::parse(&setting.input)?;
        }
        Trigger::parse(&self.recording.cancel_key)?;
        Replacer::new(&self.replacements)?;
        if !(0.0..=1.0).contains(&self.sounds.volume) {
            return Err(Error::Config("sounds.volume: between 0 and 1".into()));
        }
        if !(0.1..=1.0).contains(&self.overlay.opacity) {
            return Err(Error::Config("overlay.opacity: between 0.1 and 1".into()));
        }
        if !(self.overlay.position.is_empty() || self.overlay.position.len() == 2) {
            return Err(Error::Config("overlay.position: [x, y] or []".into()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_valid() {
        let config = Config::parse(DEFAULT_CONFIG).unwrap();
        assert_eq!(config.server.urls.len(), 2);
        assert_eq!(config.transcription.device, "auto");
        assert_eq!(config.recording.stop_after_silence_s, 0);
        assert_eq!(config.recording.max_duration_s, 0);
    }

    #[test]
    fn the_defaults_survive_a_save() {
        let config = Config::parse(DEFAULT_CONFIG).unwrap();
        let again = Config::parse(&toml::to_string_pretty(&config).unwrap()).unwrap();
        assert_eq!(config, again);
    }

    #[test]
    fn an_unknown_field_is_an_error() {
        let text = DEFAULT_CONFIG.replace("[history]\n", "[history]\nsizes = 3\n");
        assert!(Config::parse(&text).is_err());
    }

    #[test]
    fn a_missing_field_is_an_error() {
        let text = DEFAULT_CONFIG.replace("connect_timeout_ms = 1500\n", "");
        assert!(Config::parse(&text).is_err());
    }

    #[test]
    fn a_program_can_have_its_own_method() {
        let mut config = Config::parse(DEFAULT_CONFIG).unwrap();
        config.insert.per_program.push(ProgramMethod { process: "mstsc.exe".into(), method: InsertMethod::Clipboard });
        assert_eq!(config.insert.method_for("MSTSC.EXE"), InsertMethod::Clipboard);
        assert_eq!(config.insert.method_for("notepad.exe"), InsertMethod::Type);
    }

    #[test]
    fn a_trigger_that_cannot_be_read_is_an_error() {
        let text = DEFAULT_CONFIG.replace("input = \"F9\"", "input = \"Mouse:Thumb\"");
        assert!(Config::parse(&text).is_err());
    }
}
