//! Hotkey registration, event loop, push-to-talk state machine

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use once_cell::sync::Lazy;
use tauri::AppHandle;

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

    tracing::info!("Hotkey manager initialized");
    Ok(())
}

/// Register a hotkey binding. Must be called from the main thread.
pub fn register(binding: &HotkeyBinding) -> Result<(), AppError> {
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

    tracing::info!(
        "Registered hotkey: {} + {}",
        binding.modifiers.join("+"),
        binding.key
    );
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

        tracing::info!("Unregistered hotkey");
    }

    Ok(())
}

/// Start the hotkey event loop in a background thread
fn start_event_loop() {
    std::thread::spawn(|| {
        let receiver = GlobalHotKeyEvent::receiver();
        tracing::debug!("Hotkey event loop started");
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
            tracing::debug!("Already recording, ignoring press");
            return;
        }
    }

    // Guard: transcription in progress
    if IS_TRANSCRIBING.load(Ordering::SeqCst) {
        tracing::info!("Transcription in progress, ignoring press");
        tray::send_info_notification(
            "Transcription In Progress",
            "Please wait for the current transcription to finish.",
        );
        return;
    }

    tracing::info!("Hotkey pressed — starting recording");

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
                    tracing::warn!("Max recording duration reached, auto-stopping");
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
            tracing::error!("Failed to start recording: {}", e);

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
    let recorder = {
        let mut active = ACTIVE_RECORDER.lock().unwrap();
        active.take()
    };

    let Some(recorder) = recorder else {
        // No active recording (edge case: release without press)
        return;
    };

    // Check minimum duration
    let recording_duration = {
        let start = RECORDING_START.lock().unwrap();
        start.map(|s| s.elapsed()).unwrap_or_default()
    };

    if recording_duration < MIN_RECORDING_DURATION {
        tracing::debug!(
            "Recording too short ({:?}), skipping transcription",
            recording_duration
        );
        // Just stop the recorder, discard samples
        let _ = recorder.stop();
        tray::set_recording_state(false);
        return;
    }

    tracing::info!(
        "Stopping recording ({:?})",
        recording_duration
    );

    // Stop, encode, transcribe in a background thread to avoid blocking the event loop
    std::thread::spawn(move || {
        IS_TRANSCRIBING.store(true, Ordering::SeqCst);

        let pipeline_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let result = process_and_transcribe(recorder);

            match result {
                Ok(text) => {
                    tracing::info!("Transcription complete: {} chars", text.len());
                    tracing::debug!("Transcription text: {}", &text[..text.len().min(200)]);

                    if let Err(e) = crate::output::clipboard::set_clipboard_text(&text) {
                        tracing::error!("Clipboard error: {}", e);
                        tray::send_notification(
                            "Output Error",
                            &format!("Failed to copy to clipboard: {}", e),
                        );
                        return;
                    }

                    // Small delay to ensure clipboard is ready before paste
                    std::thread::sleep(std::time::Duration::from_millis(50));

                    if let Err(e) = crate::output::paste::simulate_paste() {
                        tracing::error!("Paste error: {}", e);
                        tray::send_notification(
                            "Paste Failed",
                            "Text was copied to clipboard but paste simulation failed. Use Ctrl+V to paste manually.",
                        );
                    }
                }
                Err(e) => {
                    let (title, body) = categorize_error(&e);
                    tracing::error!("Pipeline error: {}", e);
                    tray::send_notification(title, &body);
                }
            }
        }));

        if pipeline_result.is_err() {
            tracing::error!("Unexpected panic in transcription pipeline");
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

    tracing::info!(
        "Recorded {} samples at {} Hz, {} ch",
        samples.len(),
        sample_rate,
        channels
    );

    // Encode using the format selected in settings, with fallback
    let preferred_format = {
        crate::SETTINGS
            .read()
            .map(|s| s.audio_format.clone())
            .unwrap_or(crate::config::schema::AudioFormat::Opus)
    };

    let (audio_data, mime_type) = match preferred_format {
        crate::config::schema::AudioFormat::Opus => {
            match audio::encoder::encode_to_opus(&samples, sample_rate, channels) {
                Ok(data) => {
                    tracing::info!("Encoded to Opus: {} bytes", data.len());
                    (data, audio::encoder::opus_mime_type())
                }
                Err(e) => {
                    tracing::warn!("Opus encoding failed, falling back to WAV: {}", e);
                    let wav_data =
                        audio::encoder::encode_to_wav(&samples, sample_rate, channels)?;
                    tracing::info!("Encoded to WAV (fallback): {} bytes", wav_data.len());
                    (wav_data, audio::encoder::wav_mime_type())
                }
            }
        }
        crate::config::schema::AudioFormat::Wav => {
            match audio::encoder::encode_to_wav(&samples, sample_rate, channels) {
                Ok(data) => {
                    tracing::info!("Encoded to WAV: {} bytes", data.len());
                    (data, audio::encoder::wav_mime_type())
                }
                Err(e) => {
                    tracing::warn!("WAV encoding failed, falling back to Opus: {}", e);
                    let opus_data =
                        audio::encoder::encode_to_opus(&samples, sample_rate, channels)?;
                    tracing::info!("Encoded to Opus (fallback): {} bytes", opus_data.len());
                    (opus_data, audio::encoder::opus_mime_type())
                }
            }
        }
    };

    // Transcribe via provider pool using the active preset's system prompt
    let system_prompt = crate::active_system_prompt();

    let pool = crate::PROVIDER_POOL
        .read()
        .map_err(|_| AppError::Transcription("Failed to lock provider pool".to_string()))?;

    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| AppError::Transcription(format!("Failed to create runtime: {}", e)))?;

    tracing::info!("Sending audio to AI provider...");
    let result = rt.block_on(pool.transcribe(&audio_data, mime_type, &system_prompt))?;

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

/// Format a hotkey binding for display
pub fn format_hotkey(binding: &HotkeyBinding) -> String {
    let mut parts: Vec<&str> = binding.modifiers.iter().map(|s| s.as_str()).collect();
    parts.push(&binding.key);
    parts.join(" + ")
}

/// Categorize an AppError into a user-friendly notification title and body
fn categorize_error(error: &AppError) -> (&'static str, String) {
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
        _ => "Error",
    };
    (title, body)
}
