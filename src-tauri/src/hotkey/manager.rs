//! Hotkey registration, event loop, push-to-talk state machine

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use once_cell::sync::Lazy;
use tauri::AppHandle;

use tracing::{debug, error, info};

use super::conflict::HotkeyBinding;
use super::parse::{parse_code, parse_modifiers};
use crate::audio;
use crate::error::AppError;
use crate::tray;

/// Minimum recording duration to trigger transcription (50ms)
const MIN_RECORDING_DURATION: Duration = Duration::from_millis(50);

/// Debounce window for toggle mode to prevent key-repeat from rapidly toggling
const TOGGLE_DEBOUNCE: Duration = Duration::from_millis(200);

// Thread-local hotkey manager - must be used on the main thread only
thread_local! {
    static MANAGER: RefCell<Option<GlobalHotKeyManager>> = const { RefCell::new(None) };
}

/// Currently registered hotkey (id, HotKey object)
static REGISTRY: Lazy<Mutex<Option<(u32, HotKey)>>> = Lazy::new(|| Mutex::new(None));

/// Flag to track if event loop is running
static EVENT_LOOP_RUNNING: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

/// Active audio recorder handle
static ACTIVE_RECORDER: Lazy<Mutex<Option<audio::recorder::AudioRecorderHandle>>> =
    Lazy::new(|| Mutex::new(None));

/// Recording start time (for minimum duration check)
static RECORDING_START: Lazy<Mutex<Option<Instant>>> = Lazy::new(|| Mutex::new(None));

/// Flag indicating a transcription API call is in-flight
static IS_TRANSCRIBING: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

/// App handle for use in the event loop
static APP_HANDLE: Lazy<Mutex<Option<AppHandle>>> = Lazy::new(|| Mutex::new(None));

/// Last press timestamp for toggle debounce
static LAST_TOGGLE_PRESS: Lazy<Mutex<Option<Instant>>> = Lazy::new(|| Mutex::new(None));

/// Initialize the hotkey manager on the main thread
pub fn init(app: &AppHandle) -> Result<(), AppError> {
    // Store the app handle
    {
        let mut handle = APP_HANDLE.lock().unwrap();
        *handle = Some(app.clone());
    }

    MANAGER.with(|m| {
        let mut manager_ref = m.borrow_mut();
        if manager_ref.is_some() {
            return Ok::<(), AppError>(());
        }

        let manager = GlobalHotKeyManager::new()
            .map_err(|e| AppError::Hotkey(format!("Failed to create hotkey manager: {}", e)))?;

        *manager_ref = Some(manager);
        Ok::<(), AppError>(())
    })?;

    // Start event loop if not already running
    if !EVENT_LOOP_RUNNING.swap(true, Ordering::SeqCst) {
        start_event_loop();
    }

    Ok(())
}

/// Register a hotkey binding. Must be called from the main thread.
pub fn register(binding: &HotkeyBinding) -> Result<(), AppError> {
    info!(key = %binding.key, modifiers = ?binding.modifiers, "Registering hotkey");

    // Unregister existing hotkey first
    unregister()?;

    let hotkey = parse_hotkey(binding)?;
    let hotkey_id = hotkey.id();

    MANAGER.with(|m| {
        let manager_ref = m.borrow();
        if let Some(manager) = manager_ref.as_ref() {
            manager
                .register(hotkey)
                .map_err(|e| AppError::Hotkey(format!("Failed to register hotkey: {}", e)))?;
        }
        Ok::<(), AppError>(())
    })?;

    let mut registry = REGISTRY.lock().unwrap();
    *registry = Some((hotkey_id, hotkey));

    info!("Hotkey registered successfully");
    Ok(())
}

/// Unregister the current hotkey. Must be called from the main thread.
pub fn unregister() -> Result<(), AppError> {
    let mut registry = REGISTRY.lock().unwrap();

    if let Some((_, hotkey)) = registry.take() {
        MANAGER.with(|m| {
            let manager_ref = m.borrow();
            if let Some(manager) = manager_ref.as_ref() {
                manager.unregister(hotkey).map_err(|e| {
                    AppError::Hotkey(format!("Failed to unregister hotkey: {}", e))
                })?;
            }
            Ok::<(), AppError>(())
        })?;

    }

    Ok(())
}

/// Start the hotkey event loop in a background thread
fn start_event_loop() {
    std::thread::spawn(|| {
        let receiver = GlobalHotKeyEvent::receiver();
        loop {
            if let Ok(event) = receiver.recv() {
                let registry = REGISTRY.lock().unwrap();
                if let Some((registered_id, _)) = registry.as_ref() {
                    if *registered_id == event.id {
                        let mode = crate::SETTINGS
                            .read()
                            .map(|s| s.recording_mode.clone())
                            .unwrap_or(crate::config::schema::RecordingMode::HoldToRecord);

                        match event.state() {
                            HotKeyState::Pressed => {
                                match mode {
                                    crate::config::schema::RecordingMode::HoldToRecord => {
                                        handle_hotkey_press();
                                    }
                                    crate::config::schema::RecordingMode::Toggle => {
                                        // Debounce: ignore rapid repeated presses (key repeat)
                                        {
                                            let mut last = LAST_TOGGLE_PRESS.lock().unwrap();
                                            if let Some(prev) = *last {
                                                if prev.elapsed() < TOGGLE_DEBOUNCE {
                                                    continue;
                                                }
                                            }
                                            *last = Some(Instant::now());
                                        }

                                        let is_recording = {
                                            let recorder = ACTIVE_RECORDER.lock().unwrap();
                                            recorder.is_some()
                                        };

                                        if is_recording {
                                            stop_and_transcribe();
                                        } else {
                                            handle_hotkey_press();
                                        }
                                    }
                                }
                            }
                            HotKeyState::Released => {
                                match mode {
                                    crate::config::schema::RecordingMode::HoldToRecord => {
                                        stop_and_transcribe();
                                    }
                                    crate::config::schema::RecordingMode::Toggle => {
                                        // Ignore release in toggle mode
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    });
}

/// Handle hotkey press — start recording
fn handle_hotkey_press() {
    // Guard: already recording, ignore
    {
        let recorder = ACTIVE_RECORDER.lock().unwrap();
        if recorder.is_some() {
            return;
        }
    }

    // Guard: transcription in progress
    if IS_TRANSCRIBING.load(Ordering::SeqCst) {
        tray::send_info_notification(
            "Transcription In Progress",
            "Please wait for the current transcription to finish.",
        );
        return;
    }

    info!("Recording started");
    match audio::recorder::AudioRecorderHandle::start() {
        Ok(recorder) => {
            {
                let mut active = ACTIVE_RECORDER.lock().unwrap();
                *active = Some(recorder);
            }
            {
                let mut start = RECORDING_START.lock().unwrap();
                *start = Some(Instant::now());
            }

            tray::set_recording_state(true);

            // Spawn max-duration timer using configurable duration
            let max_secs = crate::SETTINGS
                .read()
                .map(|s| s.max_recording_duration_secs)
                .unwrap_or(600);
            let max_duration = Duration::from_secs(max_secs);
            std::thread::spawn(move || {
                std::thread::sleep(max_duration);
                let has_recorder = {
                    let recorder = ACTIVE_RECORDER.lock().unwrap();
                    recorder.is_some()
                };
                if has_recorder {
                    tray::send_info_notification(
                        "Recording Auto-Stopped",
                        &format!(
                            "Maximum recording duration ({} sec) reached. Transcribing...",
                            max_secs
                        ),
                    );
                    stop_and_transcribe();
                }
            });
        }
        Err(e) => {
            error!(error = %e, "Audio recorder initialization failed");

            #[cfg(target_os = "macos")]
            if e.to_string().contains("No input device") {
                tray::send_notification(
                    "Microphone Access Required",
                    "Please grant microphone access in System Settings > Privacy & Security > Microphone",
                );
                return;
            }

            tray::send_notification("Recording Error", &e.to_string());
        }
    }
}

/// Stop recording, encode audio, and transcribe. Called from both modes and the max-duration timer.
fn stop_and_transcribe() {
    info!("Recording stopped, starting transcription pipeline");

    let recorder = {
        let mut active = ACTIVE_RECORDER.lock().unwrap();
        active.take()
    };

    let Some(recorder) = recorder else {
        debug!("No active recording to stop");
        return;
    };

    // Check minimum duration
    let recording_duration = {
        let start = RECORDING_START.lock().unwrap();
        start.map(|s| s.elapsed()).unwrap_or_default()
    };

    if recording_duration < MIN_RECORDING_DURATION {
        // Just stop the recorder, discard samples
        let _ = recorder.stop();
        tray::set_recording_state(false);
        return;
    }

    // Stop, encode, transcribe in a background thread to avoid blocking the event loop
    std::thread::spawn(move || {
        IS_TRANSCRIBING.store(true, Ordering::SeqCst);

        let pipeline_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let result = process_and_transcribe(recorder);

            match result {
                Ok(text) => {
                    info!("Transcription complete, pasting result");
                    if let Err(e) = crate::output::clipboard::set_clipboard_text(&text) {
                        tray::send_notification(
                            "Output Error",
                            &format!("Failed to copy to clipboard: {}", e),
                        );
                        return;
                    }

                    // Small delay to ensure clipboard is ready before paste
                    std::thread::sleep(std::time::Duration::from_millis(50));

                    if let Err(_e) = crate::output::paste::simulate_paste() {
                        tray::send_notification(
                            "Paste Failed",
                            "Text was copied to clipboard but paste simulation failed. Use Ctrl+V to paste manually.",
                        );
                    }
                }
                Err(e) => {
                    let (title, body) = categorize_error(&e);
                    tray::send_notification(title, &body);
                }
            }
        }));

        if pipeline_result.is_err() {
            tray::send_notification(
                "Unexpected Error",
                "An unexpected error occurred. Check the logs for details.",
            );
        }

        IS_TRANSCRIBING.store(false, Ordering::SeqCst);
        tray::set_recording_state(false);
    });
}

/// Stop recording, encode audio, send to AI, return transcription text
fn process_and_transcribe(
    recorder: audio::recorder::AudioRecorderHandle,
) -> Result<String, AppError> {
    let (samples, sample_rate, channels) = recorder.stop()?;

    if samples.is_empty() {
        return Err(AppError::Audio("No audio recorded".to_string()));
    }

    let mode = {
        crate::SETTINGS
            .read()
            .map(|s| s.transcription_mode.clone())
            .unwrap_or(crate::config::schema::TranscriptionMode::Cloud)
    };

    debug!(mode = ?mode, sample_count = samples.len(), sample_rate, channels, "Processing audio");

    match mode {
        crate::config::schema::TranscriptionMode::Local => {
            transcribe_local(&samples, sample_rate, channels)
        }
        crate::config::schema::TranscriptionMode::Cloud => {
            transcribe_cloud(&samples, sample_rate, channels)
        }
    }
}

/// Transcribe via local Whisper engine
fn transcribe_local(
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
) -> Result<String, AppError> {
    // Reject dead silence (all zeros) before expensive Whisper inference.
    // Use a very low threshold to catch only truly silent/dead input devices.
    let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
    if rms < 0.0001 {
        return Err(AppError::Audio("No speech detected (audio is silent)".into()));
    }

    debug!("Resampling audio for Whisper");
    let resampled = audio::encoder::resample_for_whisper(samples, sample_rate, channels)?;

    // Get the app handle for model path resolution
    let app_handle = {
        let handle = APP_HANDLE.lock().unwrap();
        handle
            .clone()
            .ok_or_else(|| AppError::Transcription("App handle not available".into()))?
    };

    debug!("Ensuring Whisper model is loaded");
    crate::ensure_whisper_loaded(&app_handle)?;

    let (language, translate) = {
        let settings = crate::SETTINGS
            .read()
            .map_err(|e| AppError::Config(e.to_string()))?;
        let lang = match settings.whisper_config.language {
            crate::config::schema::WhisperLanguage::Auto => "auto",
            crate::config::schema::WhisperLanguage::German => "de",
            crate::config::schema::WhisperLanguage::English => "en",
        }
        .to_string();
        (lang, settings.whisper_config.translate_to_english)
    };

    let engine = crate::WHISPER_ENGINE
        .read()
        .map_err(|e| AppError::Transcription(e.to_string()))?;
    let engine = engine
        .as_ref()
        .ok_or_else(|| AppError::Transcription("Whisper engine not loaded".into()))?;
    info!(language = %language, translate, "Running Whisper inference");
    engine.transcribe(&resampled, &language, translate)
}

/// Transcribe via cloud provider pool
fn transcribe_cloud(
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
) -> Result<String, AppError> {
    let preferred_format = {
        crate::SETTINGS
            .read()
            .map(|s| s.audio_format.clone())
            .unwrap_or(crate::config::schema::AudioFormat::Opus)
    };

    debug!(format = ?preferred_format, "Encoding audio for cloud transcription");
    let (audio_data, mime_type) = match preferred_format {
        crate::config::schema::AudioFormat::Opus => {
            match audio::encoder::encode_to_opus(samples, sample_rate, channels) {
                Ok(data) => (data, audio::encoder::opus_mime_type()),
                Err(_) => {
                    let wav_data =
                        audio::encoder::encode_to_wav(samples, sample_rate, channels)?;
                    (wav_data, audio::encoder::wav_mime_type())
                }
            }
        }
        crate::config::schema::AudioFormat::Wav => {
            match audio::encoder::encode_to_wav(samples, sample_rate, channels) {
                Ok(data) => (data, audio::encoder::wav_mime_type()),
                Err(_) => {
                    let opus_data =
                        audio::encoder::encode_to_opus(samples, sample_rate, channels)?;
                    (opus_data, audio::encoder::opus_mime_type())
                }
            }
        }
    };

    let system_prompt = crate::active_system_prompt();

    let pool = crate::PROVIDER_POOL
        .read()
        .map_err(|_| AppError::Transcription("Failed to lock provider pool".to_string()))?;

    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| AppError::Transcription(format!("Failed to create runtime: {}", e)))?;

    info!(mime_type, audio_size = audio_data.len(), "Sending audio to cloud provider");
    let result = rt.block_on(pool.transcribe(&audio_data, mime_type, &system_prompt))?;

    info!("Cloud transcription complete");
    Ok(result.text)
}

/// Parse a HotkeyBinding into a global_hotkey HotKey
fn parse_hotkey(binding: &HotkeyBinding) -> Result<HotKey, AppError> {
    let modifiers = parse_modifiers(&binding.modifiers)?;
    let code = parse_code(&binding.key)?;

    let hotkey = if modifiers.is_empty() {
        HotKey::new(None, code)
    } else {
        HotKey::new(Some(modifiers), code)
    };

    Ok(hotkey)
}

/// Categorize an AppError into a user-friendly notification title and body
fn categorize_error(error: &AppError) -> (&'static str, String) {
    error!(error = ?error, "Pipeline error");
    let body = error.to_string();
    let title = match error {
        AppError::Audio(_) => "Recording Error",
        AppError::Transcription(msg) => {
            let lower = msg.to_lowercase();
            if lower.contains("no ai providers") || lower.contains("provider") {
                "Configuration Error"
            } else if lower.contains("network")
                || lower.contains("request failed")
                || lower.contains("timed out")
            {
                "Network Error"
            } else if lower.contains("api error 401") || lower.contains("api error 403") {
                "Authentication Error"
            } else if lower.contains("api error 429") || lower.contains("quota") {
                "Rate Limit Error"
            } else {
                "Transcription Error"
            }
        }
        AppError::Output(_) => "Output Error",
        AppError::ModelDownload(_) => "Model Error",
        _ => "Error",
    };
    (title, body)
}
