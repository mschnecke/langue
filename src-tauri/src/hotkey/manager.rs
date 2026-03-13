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

/// Maximum recording duration (10 minutes)
const MAX_RECORDING_DURATION: Duration = Duration::from_secs(600);

/// Minimum recording duration to trigger transcription (0.5 seconds)
const MIN_RECORDING_DURATION: Duration = Duration::from_millis(500);

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
                        match event.state() {
                            HotKeyState::Pressed => {
                                handle_hotkey_press();
                            }
                            HotKeyState::Released => {
                                handle_hotkey_release();
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
        tray::send_notification(
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

            // Spawn max-duration timer
            std::thread::spawn(|| {
                std::thread::sleep(MAX_RECORDING_DURATION);
                let has_recorder = {
                    let recorder = ACTIVE_RECORDER.lock().unwrap();
                    recorder.is_some()
                };
                if has_recorder {
                    tracing::warn!("Max recording duration reached, auto-stopping");
                    handle_hotkey_release();
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

/// Handle hotkey release — stop recording, encode audio
fn handle_hotkey_release() {
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
        "Hotkey released — stopping recording ({:?})",
        recording_duration
    );

    // Stop and encode in a background thread to avoid blocking the event loop
    std::thread::spawn(move || {
        IS_TRANSCRIBING.store(true, Ordering::SeqCst);

        let result = process_recording(recorder);

        IS_TRANSCRIBING.store(false, Ordering::SeqCst);
        tray::set_recording_state(false);

        match result {
            Ok((audio_data, mime_type)) => {
                tracing::info!(
                    "Audio encoded: {} bytes, mime: {}",
                    audio_data.len(),
                    mime_type
                );
                // Phase 3 will send to AI provider here
                // Phase 4 will copy to clipboard and paste here
            }
            Err(e) => {
                tracing::error!("Recording pipeline error: {}", e);
                tray::send_notification("Recording Error", &e.to_string());
            }
        }
    });
}

/// Stop recording, encode audio, return (data, mime_type)
fn process_recording(
    recorder: audio::recorder::AudioRecorderHandle,
) -> Result<(Vec<u8>, &'static str), AppError> {
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

    // Try Opus first, fall back to WAV
    match audio::encoder::encode_to_opus(&samples, sample_rate, channels) {
        Ok(data) => {
            tracing::info!("Encoded to Opus: {} bytes", data.len());
            Ok((data, audio::encoder::opus_mime_type()))
        }
        Err(e) => {
            tracing::warn!("Opus encoding failed, falling back to WAV: {}", e);
            let wav_data = audio::encoder::encode_to_wav(&samples, sample_rate, channels)?;
            tracing::info!("Encoded to WAV: {} bytes", wav_data.len());
            Ok((wav_data, audio::encoder::wav_mime_type()))
        }
    }
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
