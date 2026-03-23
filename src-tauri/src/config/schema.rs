//! Configuration data structures

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default = "default_true")]
    pub start_with_system: bool,

    #[serde(default = "default_true")]
    pub show_tray_notifications: bool,

    #[serde(default = "default_hotkey")]
    pub hotkey: HotkeyConfig,

    #[serde(default = "default_audio_format")]
    pub audio_format: AudioFormat,

    #[serde(default)]
    pub presets: Vec<Preset>,

    #[serde(default = "default_active_preset_id")]
    pub active_preset_id: String,

    #[serde(default)]
    pub providers: Vec<ProviderConfig>,

    #[serde(default = "default_recording_mode")]
    pub recording_mode: RecordingMode,

    #[serde(default = "default_max_recording_duration_secs")]
    pub max_recording_duration_secs: u64,

    #[serde(default)]
    pub transcription_mode: TranscriptionMode,

    #[serde(default)]
    pub whisper_config: WhisperConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub system_prompt: String,
    #[serde(default)]
    pub is_builtin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    pub id: String,
    pub provider_type: ProviderType,
    pub api_key: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyConfig {
    pub modifiers: Vec<String>,
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioFormat {
    Opus,
    Wav,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum RecordingMode {
    HoldToRecord,
    Toggle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    Gemini,
    #[serde(rename = "openai")]
    OpenAi,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum TranscriptionMode {
    Cloud,
    Local,
}

impl Default for TranscriptionMode {
    fn default() -> Self {
        TranscriptionMode::Cloud
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum WhisperLanguage {
    Auto,
    German,
    English,
}

impl Default for WhisperLanguage {
    fn default() -> Self {
        WhisperLanguage::German
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhisperConfig {
    #[serde(default = "default_whisper_model")]
    pub selected_model: String,
    #[serde(default)]
    pub language: WhisperLanguage,
    #[serde(default = "default_true")]
    pub translate_to_english: bool,
}

impl Default for WhisperConfig {
    fn default() -> Self {
        Self {
            selected_model: default_whisper_model(),
            language: WhisperLanguage::default(),
            translate_to_english: true,
        }
    }
}

fn default_whisper_model() -> String {
    "large-v3".to_string()
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            start_with_system: true,
            show_tray_notifications: true,
            hotkey: default_hotkey(),
            audio_format: AudioFormat::Opus,
            presets: super::presets::get_builtin_presets(),
            active_preset_id: default_active_preset_id(),
            providers: Vec::new(),
            recording_mode: default_recording_mode(),
            max_recording_duration_secs: default_max_recording_duration_secs(),
            transcription_mode: TranscriptionMode::default(),
            whisper_config: WhisperConfig::default(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_hotkey() -> HotkeyConfig {
    HotkeyConfig {
        #[cfg(target_os = "macos")]
        modifiers: vec!["Cmd".to_string(), "Shift".to_string()],
        #[cfg(not(target_os = "macos"))]
        modifiers: vec!["Ctrl".to_string(), "Shift".to_string()],
        key: "Space".to_string(),
    }
}

fn default_audio_format() -> AudioFormat {
    AudioFormat::Opus
}

fn default_active_preset_id() -> String {
    "de-transcribe".to_string()
}

fn default_recording_mode() -> RecordingMode {
    RecordingMode::HoldToRecord
}

fn default_max_recording_duration_secs() -> u64 {
    600 // 10 minutes
}
