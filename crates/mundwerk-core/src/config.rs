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
    pub device: Device,
    pub language: String,
}

/// Where the STT-Suite transcribes (its `device` field).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Device {
    Auto,
    Cuda,
    Cpu,
}

/// A percentage of the settings as a fraction, 0 to 1.
#[must_use]
pub fn fraction(percent: u8) -> f32 {
    f32::from(percent) / 100.0
}

impl Device {
    /// As the STT-Suite's form takes it.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Cuda => "cuda",
            Self::Cpu => "cpu",
        }
    }
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
    pub silence_level_percent: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sounds {
    pub enabled: bool,
    pub start: String,
    pub stop: String,
    pub cancel: String,
    pub volume_percent: u8,
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
    pub clipboard_restore_delay_ms: u64,
    /// Empty = off.
    pub again_key: String,
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
    pub opacity_percent: u8,
    pub error_shown_s: u64,
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
    pub language: crate::i18n::LanguageSetting,
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

    /// Check the settings and write them to `path` (the settings window and the tray's overlay switch save this
    /// way). The file keeps the explanations of `default_config.toml`: see [`Config::to_text`].
    ///
    /// # Errors
    /// [`Config::validate`] refuses them (nothing is written then), or the file cannot be written.
    pub fn save(&self, path: &Path) -> Result<(), Error> {
        self.validate()?;
        std::fs::write(path, self.to_text()?)?;
        Ok(())
    }

    /// The settings as a file: the values put into `default_config.toml`, so every setting keeps its comment.
    ///
    /// # Errors
    /// Only if the settings cannot be written as TOML, which their types rule out.
    pub fn to_text(&self) -> Result<String, Error> {
        let error = |error: &dyn std::fmt::Display| Error::Config(error.to_string());
        let mut template: toml_edit::DocumentMut = DEFAULT_CONFIG.parse().map_err(|e| error(&e))?;
        let values: toml_edit::DocumentMut =
            toml::to_string(self).map_err(|e| error(&e))?.parse().map_err(|e| error(&e))?;
        merge(template.as_table_mut(), values.as_table());
        Ok(template.to_string())
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
        if !self.insert.again_key.is_empty() {
            Trigger::parse(&self.insert.again_key)?;
        }
        Replacer::new(&self.replacements)?;
        for (name, value, lowest) in [
            ("recording.silence_level_percent", self.recording.silence_level_percent, 0),
            ("sounds.volume_percent", self.sounds.volume_percent, 0),
            // An overlay below this could not be seen at all.
            ("overlay.opacity_percent", self.overlay.opacity_percent, 10),
        ] {
            if !(lowest..=100).contains(&value) {
                return Err(Error::Config(format!("{name}: between {lowest} and 100")));
            }
        }
        Ok(())
    }
}

/// Put the values of `from` into `into` and keep the comments of `into`: a value keeps its place and trailing
/// comment, a list of tables (triggers, replacements) the comment above its first entry. What `from` lacks goes.
fn merge(into: &mut toml_edit::Table, from: &toml_edit::Table) {
    let gone: Vec<String> = into.iter().map(|(key, _)| key.to_owned()).filter(|key| !from.contains_key(key)).collect();
    for key in gone {
        into.remove(&key);
    }
    for (key, item) in from {
        match (into.get_mut(key), item) {
            (Some(toml_edit::Item::Table(target)), toml_edit::Item::Table(source)) => merge(target, source),
            (Some(toml_edit::Item::Value(target)), toml_edit::Item::Value(source)) => {
                let decor = target.decor().clone();
                *target = source.clone();
                *target.decor_mut() = decor;
            }
            (Some(toml_edit::Item::ArrayOfTables(target)), toml_edit::Item::ArrayOfTables(source)) => {
                let comment = target.get(0).map(|first| first.decor().clone());
                *target = source.clone();
                if let (Some(comment), Some(first)) = (comment, target.get_mut(0)) {
                    *first.decor_mut() = comment;
                }
            }
            _ => {
                into.insert(key, item.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_valid() {
        let config = Config::parse(DEFAULT_CONFIG).unwrap();
        assert_eq!(config.server.urls.len(), 2);
        assert_eq!(config.transcription.device, Device::Auto);
        assert_eq!(config.recording.stop_after_silence_s, 0);
        assert_eq!(config.recording.max_duration_s, 0);
    }

    #[test]
    fn the_defaults_are_written_as_the_file_is() {
        assert_eq!(Config::parse(DEFAULT_CONFIG).unwrap().to_text().unwrap(), DEFAULT_CONFIG);
    }

    #[test]
    fn changed_values_keep_their_comments() {
        let mut config = Config::parse(DEFAULT_CONFIG).unwrap();
        config.overlay.mode = OverlayMode::TrayOnly;
        config.server.token = "abc".into();
        assert_eq!(
            config.to_text().unwrap(),
            DEFAULT_CONFIG
                .replace("mode = \"floating\"", "mode = \"tray_only\"")
                .replace("token = \"\"", "token = \"abc\"")
        );
    }

    #[test]
    fn lists_of_tables_are_written_anew() {
        let mut config = Config::parse(DEFAULT_CONFIG).unwrap();
        config.triggers.truncate(1);
        config.triggers[0].input = "Ctrl+Shift+Alt+D".into();
        config.replacements.clear();
        config.insert.per_program.push(ProgramMethod { process: "mstsc.exe".into(), method: InsertMethod::Clipboard });
        let text = config.to_text().unwrap();
        assert_eq!(Config::parse(&text).unwrap(), config);
        assert!(text.contains("# What starts a dictation"), "{text}");
    }

    #[test]
    fn saving_refuses_settings_that_cannot_work() {
        let folder = std::env::temp_dir().join(format!("mundwerk-test-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("config.toml");
        let mut config = Config::parse(DEFAULT_CONFIG).unwrap();
        config.save(&path).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), DEFAULT_CONFIG);
        config.server.urls.clear();
        assert!(config.save(&path).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), DEFAULT_CONFIG);
        std::fs::remove_dir_all(&folder).unwrap();
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
