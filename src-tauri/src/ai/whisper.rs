//! Local Whisper transcription engine
//!
//! All whisper-rs operations (model load, inference) run on a GCD serial
//! dispatch queue on macOS. Apple's Accelerate framework (BLAS) uses dispatch
//! queues internally and asserts correct queue context. A bare std::thread
//! lacks this context, causing `_dispatch_assert_queue_fail`.

use std::path::PathBuf;
use std::sync::mpsc;

use crate::error::AppError;

/// Commands sent to the dedicated Whisper worker.
enum WorkerCmd {
    Load {
        model_path: PathBuf,
        reply: mpsc::Sender<Result<(), AppError>>,
    },
    Transcribe {
        samples: Vec<f32>,
        language: String,
        translate: bool,
        reply: mpsc::Sender<Result<String, AppError>>,
    },
    Unload {
        reply: mpsc::Sender<()>,
    },
}

/// Thread-safe handle to the Whisper worker.
pub struct WhisperEngine {
    tx: mpsc::Sender<WorkerCmd>,
    loaded_model_id: String,
}

impl WhisperEngine {
    /// Spawns a worker, loads the model on it, and returns a handle.
    pub fn load(model_path: &std::path::Path, model_id: &str) -> Result<Self, AppError> {
        let (tx, rx) = mpsc::channel::<WorkerCmd>();

        spawn_worker(rx)?;

        // Send load command
        let (reply_tx, reply_rx) = mpsc::channel();
        tx.send(WorkerCmd::Load {
            model_path: model_path.to_path_buf(),
            reply: reply_tx,
        })
        .map_err(|e| AppError::Transcription(format!("Worker channel closed: {e}")))?;

        reply_rx
            .recv()
            .map_err(|e| AppError::Transcription(format!("Worker reply failed: {e}")))??;

        Ok(Self {
            tx,
            loaded_model_id: model_id.to_string(),
        })
    }

    pub fn transcribe(
        &self,
        samples: &[f32],
        language: &str,
        translate: bool,
    ) -> Result<String, AppError> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send(WorkerCmd::Transcribe {
                samples: samples.to_vec(),
                language: language.to_string(),
                translate,
                reply: reply_tx,
            })
            .map_err(|e| AppError::Transcription(format!("Worker channel closed: {e}")))?;

        reply_rx
            .recv()
            .map_err(|e| AppError::Transcription(format!("Worker reply failed: {e}")))?
    }

    pub fn loaded_model_id(&self) -> &str {
        &self.loaded_model_id
    }
}

impl Drop for WhisperEngine {
    fn drop(&mut self) {
        let (reply_tx, reply_rx) = mpsc::channel();
        let _ = self.tx.send(WorkerCmd::Unload { reply: reply_tx });
        let _ = reply_rx.recv();
    }
}

// ── macOS: run worker on a GCD serial dispatch queue ─────────────

#[cfg(target_os = "macos")]
fn spawn_worker(rx: mpsc::Receiver<WorkerCmd>) -> Result<(), AppError> {
    use std::ffi::c_void;

    extern "C" {
        fn dispatch_queue_create(
            label: *const u8,
            attr: *const c_void,
        ) -> *mut c_void;
        fn dispatch_async_f(
            queue: *mut c_void,
            context: *mut c_void,
            work: extern "C" fn(*mut c_void),
        );
    }

    extern "C" fn trampoline(context: *mut c_void) {
        let rx = unsafe { *Box::from_raw(context as *mut mpsc::Receiver<WorkerCmd>) };
        worker_loop(rx);
    }

    let rx_ptr = Box::into_raw(Box::new(rx)) as *mut c_void;

    unsafe {
        let queue = dispatch_queue_create(
            b"net.pisum.whisper\0".as_ptr(),
            std::ptr::null(),
        );
        dispatch_async_f(queue, rx_ptr, trampoline);
    }

    Ok(())
}

// ── Other platforms: plain thread ────────────────────────────────

#[cfg(not(target_os = "macos"))]
fn spawn_worker(rx: mpsc::Receiver<WorkerCmd>) -> Result<(), AppError> {
    std::thread::Builder::new()
        .name("whisper-worker".into())
        .spawn(move || {
            worker_loop(rx);
        })
        .map_err(|e| AppError::Transcription(format!("Failed to spawn Whisper thread: {e}")))?;
    Ok(())
}

// ── Worker loop (platform-independent) ───────────────────────────

fn worker_loop(rx: mpsc::Receiver<WorkerCmd>) {
    use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

    let mut ctx: Option<WhisperContext> = None;

    while let Ok(cmd) = rx.recv() {
        match cmd {
            WorkerCmd::Load {
                model_path,
                reply,
            } => {
                let result = (|| -> Result<(), AppError> {
                    let params = WhisperContextParameters::default();
                    let path_str = model_path
                        .to_str()
                        .ok_or_else(|| AppError::Transcription("Invalid model path".into()))?;
                    let new_ctx = WhisperContext::new_with_params(path_str, params).map_err(
                        |e| AppError::Transcription(format!("Failed to load Whisper model: {e}")),
                    )?;
                    ctx = Some(new_ctx);
                    Ok(())
                })();
                let _ = reply.send(result);
            }
            WorkerCmd::Transcribe {
                samples,
                language,
                translate,
                reply,
            } => {
                let result = (|| -> Result<String, AppError> {
                    let whisper_ctx = ctx.as_ref().ok_or_else(|| {
                        AppError::Transcription("Whisper model not loaded".into())
                    })?;

                    let mut state = whisper_ctx.create_state().map_err(|e| {
                        AppError::Transcription(format!("Failed to create Whisper state: {e}"))
                    })?;

                    let mut params =
                        FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

                    if language == "auto" {
                        params.set_language(None);
                    } else {
                        params.set_language(Some(&language));
                    }
                    params.set_translate(translate);
                    params.set_no_timestamps(true);
                    params.set_suppress_blank(true);
                    params.set_suppress_nst(true);
                    params.set_print_special(false);
                    params.set_print_progress(false);
                    params.set_print_realtime(false);

                    state.full(params, &samples).map_err(|e| {
                        AppError::Transcription(format!("Whisper inference failed: {e}"))
                    })?;

                    let num_segments = state.full_n_segments();

                    let mut text = String::new();
                    for i in 0..num_segments {
                        if let Some(segment) = state.get_segment(i) {
                            if let Ok(s) = segment.to_str_lossy() {
                                text.push_str(s.trim());
                                if i < num_segments - 1 {
                                    text.push(' ');
                                }
                            }
                        }
                    }

                    Ok(text.trim().to_string())
                })();
                let _ = reply.send(result);
            }
            WorkerCmd::Unload { reply } => {
                drop(ctx);
                let _ = reply.send(());
                return;
            }
        }
    }
}
