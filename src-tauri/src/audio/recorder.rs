//! Audio capture via cpal on a dedicated thread

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use tracing::{debug, error, info};

use crate::error::AppError;

/// Message sent to the recording thread
enum RecorderCommand {
    Stop,
}

/// Audio recorder that captures from the default input device.
/// Uses a dedicated thread because the cpal stream is not `Send`.
pub struct AudioRecorderHandle {
    command_tx: Sender<RecorderCommand>,
    samples: Arc<Mutex<Vec<f32>>>,
    _is_recording: Arc<AtomicBool>,
    sample_rate: u32,
    channels: u16,
    thread_handle: Option<JoinHandle<()>>,
}

impl AudioRecorderHandle {
    /// Start a new recording session on the default input device.
    pub fn start() -> Result<Self, AppError> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| AppError::Audio("No input device found".to_string()))?;

        debug!(device = ?device.name().unwrap_or_default(), "Using input device");

        let config = device
            .default_input_config()
            .map_err(|e| AppError::Audio(format!("Failed to get input config: {}", e)))?;

        let sample_rate = config.sample_rate().0;
        let channels = config.channels();
        info!(sample_rate, channels, format = ?config.sample_format(), "Audio recording started");
        let samples = Arc::new(Mutex::new(Vec::new()));
        let is_recording = Arc::new(AtomicBool::new(true));

        let (command_tx, command_rx) = mpsc::channel::<RecorderCommand>();

        let samples_clone = Arc::clone(&samples);
        let is_recording_clone = Arc::clone(&is_recording);
        let config_clone = config.clone();

        let thread_handle = thread::spawn(move || {
            let host = cpal::default_host();
            let device = match host.default_input_device() {
                Some(d) => d,
                None => {
                    return;
                }
            };

            let err_fn = |err| {
                error!(error = %err, "Audio stream error");
            };

            let stream = match config_clone.sample_format() {
                cpal::SampleFormat::F32 => {
                    let samples = Arc::clone(&samples_clone);
                    let is_recording = Arc::clone(&is_recording_clone);
                    device.build_input_stream(
                        &config_clone.clone().into(),
                        move |data: &[f32], _: &cpal::InputCallbackInfo| {
                            if is_recording.load(Ordering::SeqCst) {
                                if let Ok(mut s) = samples.lock() {
                                    s.extend_from_slice(data);
                                }
                            }
                        },
                        err_fn,
                        None,
                    )
                }
                cpal::SampleFormat::I16 => {
                    let samples = Arc::clone(&samples_clone);
                    let is_recording = Arc::clone(&is_recording_clone);
                    device.build_input_stream(
                        &config_clone.clone().into(),
                        move |data: &[i16], _: &cpal::InputCallbackInfo| {
                            if is_recording.load(Ordering::SeqCst) {
                                if let Ok(mut s) = samples.lock() {
                                    for &sample in data {
                                        s.push(sample as f32 / 32768.0);
                                    }
                                }
                            }
                        },
                        err_fn,
                        None,
                    )
                }
                cpal::SampleFormat::U16 => {
                    let samples = Arc::clone(&samples_clone);
                    let is_recording = Arc::clone(&is_recording_clone);
                    device.build_input_stream(
                        &config_clone.into(),
                        move |data: &[u16], _: &cpal::InputCallbackInfo| {
                            if is_recording.load(Ordering::SeqCst) {
                                if let Ok(mut s) = samples.lock() {
                                    for &sample in data {
                                        s.push((sample as f32 - 32768.0) / 32768.0);
                                    }
                                }
                            }
                        },
                        err_fn,
                        None,
                    )
                }
                _ => {
                    return;
                }
            };

            let stream = match stream {
                Ok(s) => s,
                Err(_) => {
                    return;
                }
            };

            if stream.play().is_err() {
                return;
            }

            // Block until stop command
            let _ = command_rx.recv();

            // Stream is dropped here, stopping recording
            is_recording_clone.store(false, Ordering::SeqCst);
        });

        Ok(Self {
            command_tx,
            samples,
            _is_recording: is_recording,
            sample_rate,
            channels,
            thread_handle: Some(thread_handle),
        })
    }

    /// Stop recording and return (samples, sample_rate, channels).
    pub fn stop(mut self) -> Result<(Vec<f32>, u32, u16), AppError> {
        info!("Stopping audio recording");
        let _ = self.command_tx.send(RecorderCommand::Stop);

        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }

        let samples = self
            .samples
            .lock()
            .map_err(|_| AppError::Audio("Failed to lock samples".to_string()))?
            .clone();

        info!(sample_count = samples.len(), "Audio recording stopped");
        Ok((samples, self.sample_rate, self.channels))
    }
}
