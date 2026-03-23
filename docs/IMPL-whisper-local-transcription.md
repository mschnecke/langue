# Implementation Plan: Local Whisper Transcription Support

> Generated from: `docs/PRD-whisper-local-transcription.md`
> Date: 2026-03-22

## 1. Overview

This plan adds offline speech-to-text transcription via whisper.cpp (`whisper-rs` crate) as an alternative to the existing cloud provider pool (Gemini/OpenAI). Users select a transcription mode — Local (Whisper) or Cloud — and the system routes audio through the appropriate pipeline.

The Whisper engine is **not** part of the round-robin `ProviderPool`. It operates as a separate code path gated by `TranscriptionMode` in settings. The audio pipeline branches early: cloud mode encodes to Opus/WAV, while local mode resamples raw PCM to 16 kHz mono and passes it directly to whisper-rs.

Key integration points:
- `config/schema.rs` — new `TranscriptionMode` enum and `WhisperConfig` struct
- `hotkey/manager.rs` — `process_and_transcribe()` branches on mode
- New `ai/whisper.rs` module — WhisperContext lifecycle, inference
- New `whisper/` module — model download/management
- New `WhisperConfig.svelte` — settings UI for local mode

## 2. Architecture & Design

### Data Flow

```
                          ┌─────────────────────────────────┐
                          │     AudioRecorderHandle::stop() │
                          │   → (Vec<f32>, sample_rate, ch) │
                          └───────────────┬─────────────────┘
                                          │
                            ┌─────────────▼──────────────┐
                            │  TranscriptionMode check   │
                            └─────┬───────────────┬──────┘
                                  │               │
                       Cloud mode │               │ Local mode
                                  │               │
                    ┌─────────────▼─────┐   ┌─────▼──────────────┐
                    │ encode_to_opus()  │   │ resample_for_       │
                    │ or encode_to_wav()│   │ whisper()           │
                    │ → (bytes, mime)   │   │ → Vec<f32> 16kHz   │
                    └────────┬──────────┘   └─────┬──────────────┘
                             │                    │
                    ┌────────▼──────────┐   ┌─────▼──────────────┐
                    │  ProviderPool     │   │  WHISPER_ENGINE    │
                    │  .transcribe()    │   │  .transcribe()     │
                    └────────┬──────────┘   └─────┬──────────────┘
                             │                    │
                             └───────┬────────────┘
                                     │
                              ┌──────▼──────┐
                              │ paste text  │
                              └─────────────┘
```

### Module Boundaries

| Module | Responsibility |
|--------|---------------|
| `ai/whisper.rs` | WhisperContext lifecycle, inference wrapper, language config |
| `whisper/mod.rs` | Model registry (tiers, filenames, URLs, sizes) |
| `whisper/download.rs` | HTTP download with progress events, cancellation, cleanup |
| `whisper/models.rs` | Model discovery on disk, integrity checks, deletion |
| `audio/encoder.rs` | New `resample_for_whisper()` function (reuses rubato) |
| `config/schema.rs` | `TranscriptionMode`, `WhisperConfig`, `WhisperLanguage` |

### Global State

```rust
// Existing
pub static PROVIDER_POOL: Lazy<RwLock<ProviderPool>> = ...;
pub static SETTINGS: Lazy<RwLock<AppSettings>> = ...;

// New
pub static WHISPER_ENGINE: Lazy<RwLock<Option<WhisperEngine>>> = ...;
```

`WhisperEngine` wraps a `WhisperContext` and is `Option` because it's only loaded when Local mode is active and a transcription is first requested. Set to `None` when switching to Cloud mode (Req #41).

## 3. Phases & Milestones

### Phase 1: Configuration & Schema
**Goal:** Extend settings to support transcription mode selection and Whisper configuration.
**Deliverable:** Settings file can store and load `transcription_mode`, `whisper_config`. Existing users' settings deserialize without error (serde defaults).

### Phase 2: Audio Pipeline for Whisper
**Goal:** Add a resampling path that produces 16 kHz mono f32 samples from raw recorder output.
**Deliverable:** `resample_for_whisper()` function, tested independently.

### Phase 3: Whisper Engine Integration
**Goal:** Load a GGML model, run inference, return transcribed text.
**Deliverable:** `WhisperEngine` struct with `transcribe()` method. Verifiable with a hardcoded model path and test audio.

### Phase 4: Transcription Pipeline Branching
**Goal:** Route audio through cloud or local path based on `TranscriptionMode`.
**Deliverable:** End-to-end transcription works in both modes via hotkey.

### Phase 5: Model Management
**Goal:** Download, verify, list, and delete GGML models. IPC commands for frontend.
**Deliverable:** All model management IPC commands functional, progress events emitted.

### Phase 6: Settings UI
**Goal:** Mode toggle, Whisper configuration panel, model management UI.
**Deliverable:** Complete settings UI for both modes.

## 4. Files Overview

### Files to Create

| File Path | Purpose |
|-----------|---------|
| `src-tauri/src/ai/whisper.rs` | WhisperEngine: context lifecycle, inference, language config |
| `src-tauri/src/whisper/mod.rs` | Module root, model tier registry, re-exports |
| `src-tauri/src/whisper/download.rs` | HTTP model download with progress events and cancellation |
| `src-tauri/src/whisper/models.rs` | On-disk model discovery, integrity verification, deletion |
| `src/components/WhisperConfig.svelte` | Whisper settings panel (model management, language, status) |
| `src/components/ModeToggle.svelte` | Transcription mode toggle (Local vs. Cloud) |

### Files to Modify

| File Path | What Changes |
|-----------|-------------|
| `src-tauri/Cargo.toml` | Add `whisper-rs` with platform-conditional features |
| `src-tauri/src/config/schema.rs` | Add `TranscriptionMode`, `WhisperConfig`, `WhisperLanguage` |
| `src-tauri/src/config/manager.rs` | No structural changes (serde defaults handle new fields) |
| `src-tauri/src/audio/encoder.rs` | Add `resample_for_whisper()` public function |
| `src-tauri/src/ai/mod.rs` | Add `pub mod whisper;` |
| `src-tauri/src/lib.rs` | Add `WHISPER_ENGINE` global, new IPC commands, update `apply_settings()` |
| `src-tauri/src/hotkey/manager.rs` | Branch `process_and_transcribe()` on transcription mode |
| `src-tauri/src/error.rs` | Add `ModelDownload(String)` variant |
| `src-tauri/src/main.rs` | Register new IPC commands |
| `src-tauri/src/tray.rs` | No changes expected (existing notification system suffices) |
| `src/lib/types.ts` | Add `TranscriptionMode`, `WhisperConfig`, `ModelInfo` types |
| `src/lib/commands.ts` | Add model management command wrappers |
| `src/App.svelte` (or settings layout) | Add mode toggle, conditional panel rendering |
| `src/components/ProviderConfig.svelte` | Wrap in conditional (only shown in Cloud mode) |

## 5. Task Breakdown

### Phase 1: Configuration & Schema

#### Task 1.1: Add Whisper-related types to config schema

- **Files to modify:**
  - `src-tauri/src/config/schema.rs` — add new types
- **Implementation details:**

  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
  #[serde(rename_all = "camelCase")]
  pub enum TranscriptionMode {
      Cloud,
      Local,
  }

  impl Default for TranscriptionMode {
      fn default() -> Self {
          TranscriptionMode::Cloud // Req #40
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
          WhisperLanguage::Auto
      }
  }

  #[derive(Debug, Clone, Serialize, Deserialize)]
  #[serde(rename_all = "camelCase")]
  pub struct WhisperConfig {
      #[serde(default = "default_whisper_model")]
      pub selected_model: String, // e.g. "large-v3-turbo"
      #[serde(default)]
      pub language: WhisperLanguage,
  }

  fn default_whisper_model() -> String {
      "large-v3-turbo".to_string()
  }
  ```

  Add to `AppSettings`:
  ```rust
  #[serde(default)]
  pub transcription_mode: TranscriptionMode,
  #[serde(default)]
  pub whisper_config: WhisperConfig,
  ```

- **Dependencies:** None
- **Acceptance criteria:** Existing settings files deserialize without error. New fields appear with defaults when serialized. `TranscriptionMode` defaults to `Cloud`.

#### Task 1.2: Add frontend types for Whisper configuration

- **Files to modify:**
  - `src/lib/types.ts` — add TypeScript types
- **Implementation details:**

  ```typescript
  export type TranscriptionMode = 'cloud' | 'local';
  export type WhisperLanguage = 'auto' | 'german' | 'english';

  export interface WhisperConfig {
    selectedModel: string;
    language: WhisperLanguage;
  }

  export interface WhisperModelInfo {
    id: string;
    name: string;
    fileName: string;
    sizeBytes: number;
    description: string;
    downloaded: boolean;
    fileSizeOnDisk: number | null;
  }

  export interface WhisperStatus {
    state: 'ready' | 'loading' | 'noModel' | 'notActive';
    loadedModel: string | null;
  }
  ```

  Add to `AppSettings`:
  ```typescript
  transcriptionMode: TranscriptionMode;
  whisperConfig: WhisperConfig;
  ```

- **Dependencies:** Task 1.1 (types must match Rust schema)
- **Acceptance criteria:** Types compile. `AppSettings` interface matches the Rust `AppSettings` serde output.

### Phase 2: Audio Pipeline for Whisper

#### Task 2.1: Add `resample_for_whisper()` to encoder module

- **Files to modify:**
  - `src-tauri/src/audio/encoder.rs` — add public function
- **Implementation details:**

  Reuse the existing rubato `SincFixedIn` resampler pattern from `encode_to_opus()` (lines ~75-135). The new function:
  1. Mix down to mono if multi-channel (average all channels)
  2. Resample from source rate to 16000 Hz using rubato sinc interpolation
  3. Return `Vec<f32>` samples

  ```rust
  /// Resamples raw audio to 16 kHz mono f32 for Whisper inference.
  pub fn resample_for_whisper(
      samples: &[f32],
      sample_rate: u32,
      channels: u16,
  ) -> Result<Vec<f32>, AppError> {
      // Step 1: Mix to mono
      let mono = if channels > 1 {
          samples
              .chunks(channels as usize)
              .map(|frame| frame.iter().sum::<f32>() / channels as f32)
              .collect::<Vec<_>>()
      } else {
          samples.to_vec()
      };

      // Step 2: Resample to 16 kHz
      if sample_rate == 16000 {
          return Ok(mono);
      }

      let params = SincInterpolationParameters {
          sinc_len: 256,
          f_cutoff: 0.95,
          interpolation: SincInterpolationType::Linear,
          oversampling_factor: 256,
          window: WindowFunction::BlackmanHarris2,
      };
      let mut resampler = SincFixedIn::<f32>::new(
          16000.0 / sample_rate as f64,
          2.0,
          params,
          mono.len(),
          1, // mono
      ).map_err(|e| AppError::Audio(format!("Resampler init failed: {e}")))?;

      let resampled = resampler
          .process(&[&mono], None)
          .map_err(|e| AppError::Audio(format!("Resampling failed: {e}")))?;

      Ok(resampled.into_iter().next().unwrap_or_default())
  }
  ```

- **Dependencies:** None
- **Acceptance criteria:** Given 48 kHz stereo f32 samples, returns 16 kHz mono f32 samples. Given 16 kHz mono input, returns input unchanged.

### Phase 3: Whisper Engine Integration

#### Task 3.1: Add `whisper-rs` dependency with platform features

- **Files to modify:**
  - `src-tauri/Cargo.toml` — add conditional dependencies
- **Implementation details:**

  ```toml
  [target.'cfg(target_os = "macos")'.dependencies]
  whisper-rs = { version = "0.16", features = ["metal"] }

  [target.'cfg(target_os = "windows")'.dependencies]
  whisper-rs = { version = "0.16", features = ["vulkan"] }
  ```

- **Dependencies:** None
- **Acceptance criteria:** `cargo check` succeeds on the build platform.

#### Task 3.2: Create `WhisperEngine` struct and inference logic

- **Files to create:**
  - `src-tauri/src/ai/whisper.rs` — engine implementation
- **Files to modify:**
  - `src-tauri/src/ai/mod.rs` — add `pub mod whisper;`
- **Implementation details:**

  ```rust
  use whisper_rs::{WhisperContext, WhisperContextParameters, FullParams, SamplingStrategy};
  use crate::error::AppError;

  pub struct WhisperEngine {
      ctx: WhisperContext,
      loaded_model_id: String,
  }

  impl WhisperEngine {
      pub fn load(model_path: &std::path::Path, model_id: &str) -> Result<Self, AppError> {
          let params = WhisperContextParameters::default();
          let ctx = WhisperContext::new_with_params(
              model_path.to_str().ok_or_else(|| AppError::Transcription("Invalid model path".into()))?,
              params,
          ).map_err(|e| AppError::Transcription(format!("Failed to load Whisper model: {e}")))?;

          Ok(Self { ctx, loaded_model_id: model_id.to_string() })
      }

      pub fn transcribe(
          &self,
          samples: &[f32],
          language: &str, // "auto", "de", "en"
      ) -> Result<String, AppError> {
          let mut state = self.ctx.create_state()
              .map_err(|e| AppError::Transcription(format!("Failed to create Whisper state: {e}")))?;

          let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

          // Req #15, #16: translate task for English output
          params.set_translate(true);
          params.set_no_timestamps(true);
          params.set_print_special(false);
          params.set_print_progress(false);
          params.set_print_realtime(false);

          // Req #17: language hint
          if language != "auto" {
              params.set_language(Some(language));
          }

          state.full(params, samples)
              .map_err(|e| AppError::Transcription(format!("Whisper inference failed: {e}")))?;

          // Req #18: extract and concatenate all segments
          let num_segments = state.full_n_segments()
              .map_err(|e| AppError::Transcription(format!("Failed to get segments: {e}")))?;

          let mut text = String::new();
          for i in 0..num_segments {
              if let Ok(segment) = state.full_get_segment_text(i) {
                  text.push_str(segment.trim());
                  if i < num_segments - 1 {
                      text.push(' ');
                  }
              }
          }

          Ok(text.trim().to_string())
      }

      pub fn loaded_model_id(&self) -> &str {
          &self.loaded_model_id
      }
  }
  ```

  Note: `WhisperEngine` does **not** implement `TranscriptionProvider`. It has its own `transcribe(samples, language)` signature that accepts raw f32 PCM, not encoded bytes. This matches the PRD's design of keeping Whisper separate from the cloud provider pool.

- **Dependencies:** Task 3.1 (whisper-rs available), Task 2.1 (resampled audio format understood)
- **Acceptance criteria:** Given a valid GGML model path and 16 kHz mono f32 samples, returns English transcription text. Error messages are descriptive.

#### Task 3.3: Add `WHISPER_ENGINE` global state and lifecycle management

- **Files to modify:**
  - `src-tauri/src/lib.rs` — add global, load/unload helpers
- **Implementation details:**

  ```rust
  pub static WHISPER_ENGINE: Lazy<RwLock<Option<ai::whisper::WhisperEngine>>> =
      Lazy::new(|| RwLock::new(None));
  ```

  Add helper functions:
  ```rust
  /// Loads the Whisper model if not already loaded or if the selected model
  /// has changed (lazy, Req #14). Called from process_and_transcribe().
  pub fn ensure_whisper_loaded(app_handle: &AppHandle) -> Result<(), AppError> {
      let settings = SETTINGS.read().map_err(|e| AppError::Config(e.to_string()))?;
      let desired_model_id = settings.whisper_config.selected_model.clone();
      drop(settings);

      let engine = WHISPER_ENGINE.read().map_err(|e| AppError::Transcription(e.to_string()))?;
      if let Some(ref e) = *engine {
          if e.loaded_model_id() == desired_model_id {
              return Ok(()); // already loaded with correct model
          }
          // Model changed — need to reload
          drop(engine);
          unload_whisper();
      } else {
          drop(engine); // release read lock
      }

      let settings = SETTINGS.read().map_err(|e| AppError::Config(e.to_string()))?;
      let model_id = &settings.whisper_config.selected_model;
      let models_dir = app_handle.path().app_data_dir()
          .map_err(|e| AppError::Config(e.to_string()))?
          .join("models");
      let model_info = whisper::models::get_model_tier(model_id)
          .ok_or_else(|| AppError::Transcription(format!("Unknown model: {model_id}")))?;
      let model_path = models_dir.join(&model_info.file_name);

      if !model_path.exists() {
          return Err(AppError::Transcription(
              "No model downloaded. Please download a model in Settings.".into()
          )); // Req #5
      }
      drop(settings);

      let engine = ai::whisper::WhisperEngine::load(&model_path, model_id)?;
      let mut guard = WHISPER_ENGINE.write().map_err(|e| AppError::Transcription(e.to_string()))?;
      *guard = Some(engine);
      Ok(())
  }

  /// Unloads the Whisper model to free memory (Req #41).
  pub fn unload_whisper() {
      if let Ok(mut guard) = WHISPER_ENGINE.write() {
          *guard = None;
      }
  }
  ```

  Update `apply_settings()` to unload Whisper when switching to Cloud mode:
  ```rust
  // In apply_settings(), after rebuilding the provider pool:
  if settings.transcription_mode == TranscriptionMode::Cloud {
      unload_whisper();
  }
  ```

- **Dependencies:** Task 3.2
- **Acceptance criteria:** `ensure_whisper_loaded()` loads model on first call, returns `Ok` on subsequent calls. `unload_whisper()` sets engine to `None`. Switching to Cloud mode unloads.

### Phase 4: Transcription Pipeline Branching

#### Task 4.1: Branch `process_and_transcribe()` on transcription mode

- **Files to modify:**
  - `src-tauri/src/hotkey/manager.rs` — modify `process_and_transcribe()`
- **Implementation details:**

  The function currently (lines 331-384) always encodes and calls the provider pool. Add a mode check early:

  ```rust
  fn process_and_transcribe(
      recorder: AudioRecorderHandle,
      app_handle: AppHandle, // NEW: needed for model path resolution
  ) -> Result<String, AppError> {
      let (samples, sample_rate, channels) = recorder.stop()?;

      let mode = {
          let settings = crate::SETTINGS.read()
              .map_err(|e| AppError::Config(e.to_string()))?;
          settings.transcription_mode.clone()
      };

      match mode {
          TranscriptionMode::Local => {
              // Req #7, #8, #9: resample to 16 kHz mono
              let resampled = crate::audio::encoder::resample_for_whisper(
                  &samples, sample_rate, channels
              )?;

              // Req #14: lazy load model
              crate::ensure_whisper_loaded(&app_handle)?;

              // Req #17: get language setting
              let language = {
                  let settings = crate::SETTINGS.read()
                      .map_err(|e| AppError::Config(e.to_string()))?;
                  match settings.whisper_config.language {
                      WhisperLanguage::Auto => "auto",
                      WhisperLanguage::German => "de",
                      WhisperLanguage::English => "en",
                  }.to_string()
              };

              // Req #10, #19: transcribe with Whisper (no system prompt)
              let engine = crate::WHISPER_ENGINE.read()
                  .map_err(|e| AppError::Transcription(e.to_string()))?;
              let engine = engine.as_ref()
                  .ok_or_else(|| AppError::Transcription("Whisper engine not loaded".into()))?;
              engine.transcribe(&resampled, &language)
          }
          TranscriptionMode::Cloud => {
              // Existing cloud path (unchanged)
              let preferred_format = { /* read from settings */ };
              let (audio_data, mime_type) = /* encode as before */;
              let system_prompt = crate::active_system_prompt();
              let pool = crate::PROVIDER_POOL.read()
                  .map_err(|e| AppError::Transcription(e.to_string()))?;
              let rt = tokio::runtime::Runtime::new()
                  .map_err(|e| AppError::Transcription(e.to_string()))?;
              rt.block_on(pool.transcribe(&audio_data, mime_type, &system_prompt))
                  .map(|r| r.text)
          }
      }
  }
  ```

  The `AppHandle` must be threaded through from `stop_and_transcribe()`. Since the hotkey manager already has access to global state, pass `app_handle` from the Tauri setup or store it in a global.

- **Dependencies:** Task 2.1, Task 3.3
- **Acceptance criteria:** Recording → release hotkey transcribes via Whisper in Local mode, via cloud in Cloud mode. Switching mode in settings changes behavior on next transcription.

#### Task 4.2: Pass `AppHandle` to hotkey manager

- **Files to modify:**
  - `src-tauri/src/hotkey/manager.rs` — store `AppHandle` for model path resolution
  - `src-tauri/src/lib.rs` — pass `AppHandle` during hotkey manager setup
- **Implementation details:**

  Add a global or pass `AppHandle` into `start_event_loop()`. The simplest approach is a global:
  ```rust
  // In lib.rs
  pub static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();
  ```
  Set it during Tauri `setup()`. Use it in `process_and_transcribe()` to resolve `app_data_dir()`.

- **Dependencies:** None
- **Acceptance criteria:** `APP_HANDLE.get()` returns `Some` after app startup. Model path resolution works via `app_handle.path().app_data_dir()`.

### Phase 5: Model Management

#### Task 5.1: Create model registry module

- **Files to create:**
  - `src-tauri/src/whisper/mod.rs` — module root and re-exports
  - `src-tauri/src/whisper/models.rs` — model tier definitions, on-disk discovery
- **Files to modify:**
  - `src-tauri/src/main.rs` — add `mod whisper;`
- **Implementation details:**

  ```rust
  // whisper/models.rs

  pub struct ModelTier {
      pub id: &'static str,
      pub name: &'static str,
      pub file_name: &'static str,
      pub size_bytes: u64,
      pub description: &'static str,
      pub url: &'static str,
  }

  pub const MODEL_TIERS: &[ModelTier] = &[
      ModelTier {
          id: "large-v3-turbo",
          name: "Large (v3 Turbo)",
          file_name: "ggml-large-v3-turbo-q5_0.bin",
          size_bytes: 574_000_000, // ~574 MB
          description: "Best accuracy, recommended for most users",
          url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q5_0.bin",
      },
      ModelTier {
          id: "small",
          name: "Small",
          file_name: "ggml-small-q5_1.bin",
          size_bytes: 200_000_000, // ~200 MB
          description: "Lighter alternative, good accuracy",
          url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small-q5_1.bin",
      },
      ModelTier {
          id: "base",
          name: "Base",
          file_name: "ggml-base-q5_1.bin",
          size_bytes: 60_000_000, // ~60 MB
          description: "Minimal, for constrained hardware",
          url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base-q5_1.bin",
      },
  ];

  pub fn get_model_tier(id: &str) -> Option<&'static ModelTier> {
      MODEL_TIERS.iter().find(|m| m.id == id)
  }

  /// Returns model info with download status for each tier.
  pub fn list_models(models_dir: &Path) -> Vec<ModelStatus> {
      MODEL_TIERS.iter().map(|tier| {
          let path = models_dir.join(tier.file_name);
          let downloaded = path.exists();
          let file_size_on_disk = if downloaded {
              std::fs::metadata(&path).ok().map(|m| m.len())
          } else {
              None
          };
          ModelStatus {
              id: tier.id.to_string(),
              name: tier.name.to_string(),
              file_name: tier.file_name.to_string(),
              size_bytes: tier.size_bytes,
              description: tier.description.to_string(),
              downloaded,
              file_size_on_disk,
          }
      }).collect()
  }

  /// Verifies model file integrity by checking file size (Req #26).
  pub fn verify_model(models_dir: &Path, model_id: &str) -> Result<bool, AppError> {
      let tier = get_model_tier(model_id)
          .ok_or_else(|| AppError::Transcription(format!("Unknown model: {model_id}")))?;
      let path = models_dir.join(tier.file_name);
      if !path.exists() {
          return Ok(false);
      }
      let actual_size = std::fs::metadata(&path)
          .map_err(|e| AppError::Io(e))?.len();
      // Allow 5% tolerance for size differences across versions
      let expected = tier.size_bytes;
      Ok(actual_size > expected * 95 / 100 && actual_size < expected * 105 / 100)
  }

  /// Deletes a downloaded model file (Req #28).
  pub fn delete_model(models_dir: &Path, model_id: &str) -> Result<(), AppError> {
      let tier = get_model_tier(model_id)
          .ok_or_else(|| AppError::Transcription(format!("Unknown model: {model_id}")))?;
      let path = models_dir.join(tier.file_name);
      if path.exists() {
          std::fs::remove_file(&path)?;
      }
      Ok(())
  }
  ```

- **Dependencies:** None
- **Acceptance criteria:** `list_models()` returns three tiers with accurate download status. `verify_model()` checks file size. `delete_model()` removes file.

#### Task 5.2: Create model download module with progress and cancellation

- **Files to create:**
  - `src-tauri/src/whisper/download.rs` — download logic
- **Files to modify:**
  - `src-tauri/src/error.rs` — add `ModelDownload(String)` variant
- **Implementation details:**

  ```rust
  // whisper/download.rs

  use std::sync::atomic::{AtomicBool, Ordering};
  use std::sync::Arc;
  use tauri::{AppHandle, Emitter};

  /// Global cancellation flag (Req #31: only one download at a time)
  static DOWNLOAD_IN_PROGRESS: AtomicBool = AtomicBool::new(false);
  static DOWNLOAD_CANCELLED: AtomicBool = AtomicBool::new(false);

  #[derive(Clone, Serialize)]
  #[serde(rename_all = "camelCase")]
  pub struct DownloadProgress {
      pub model_id: String,
      pub bytes_downloaded: u64,
      pub total_bytes: u64,
      pub percentage: f64,
  }

  pub async fn download_model(
      app: &AppHandle,
      model_id: &str,
      models_dir: &Path,
  ) -> Result<(), AppError> {
      // Req #31: prevent concurrent downloads
      if DOWNLOAD_IN_PROGRESS.swap(true, Ordering::SeqCst) {
          return Err(AppError::ModelDownload(
              "A download is already in progress. Cancel it first.".into()
          ));
      }
      DOWNLOAD_CANCELLED.store(false, Ordering::SeqCst);

      let result = do_download(app, model_id, models_dir).await;

      DOWNLOAD_IN_PROGRESS.store(false, Ordering::SeqCst);

      if let Err(ref e) = result {
          // Req #30: clean up partial files on failure
          if let Some(tier) = super::models::get_model_tier(model_id) {
              let partial = models_dir.join(tier.file_name);
              let _ = std::fs::remove_file(&partial);
          }
      }

      result
  }

  async fn do_download(
      app: &AppHandle,
      model_id: &str,
      models_dir: &Path,
  ) -> Result<(), AppError> {
      let tier = super::models::get_model_tier(model_id)
          .ok_or_else(|| AppError::ModelDownload(format!("Unknown model: {model_id}")))?;

      std::fs::create_dir_all(models_dir)?;
      let dest = models_dir.join(tier.file_name);

      let client = reqwest::Client::new();
      let response = client.get(tier.url).send().await
          .map_err(|e| AppError::ModelDownload(format!("Download failed: {e}")))?;

      if !response.status().is_success() {
          return Err(AppError::ModelDownload(
              format!("Download failed with status: {}", response.status())
          ));
      }

      let total = response.content_length().unwrap_or(tier.size_bytes);
      let mut file = std::fs::File::create(&dest)?;
      let mut downloaded: u64 = 0;
      let mut stream = response.bytes_stream();

      use futures_util::StreamExt;
      while let Some(chunk) = stream.next().await {
          if DOWNLOAD_CANCELLED.load(Ordering::SeqCst) {
              drop(file);
              let _ = std::fs::remove_file(&dest);
              return Err(AppError::ModelDownload("Download cancelled".into()));
          }

          let chunk = chunk
              .map_err(|e| AppError::ModelDownload(format!("Network error: {e}")))?;
          std::io::Write::write_all(&mut file, &chunk)?;
          downloaded += chunk.len() as u64;

          // Emit progress every ~100KB to avoid flooding
          if downloaded % (100 * 1024) < chunk.len() as u64 {
              let _ = app.emit("whisper-download-progress", DownloadProgress {
                  model_id: model_id.to_string(),
                  bytes_downloaded: downloaded,
                  total_bytes: total,
                  percentage: (downloaded as f64 / total as f64) * 100.0,
              });
          }
      }

      // Req #26: verify file size
      if !super::models::verify_model(models_dir, model_id)? {
          let _ = std::fs::remove_file(&dest);
          return Err(AppError::ModelDownload(
              "Downloaded model failed integrity check. Please retry.".into()
          ));
      }

      Ok(())
  }

  pub fn cancel_download() {
      DOWNLOAD_CANCELLED.store(true, Ordering::SeqCst);
  }
  ```

  Add `reqwest` `stream` feature (which re-exports `futures-util` stream types — check if a separate `futures-util` dep is needed during implementation):
  ```toml
  reqwest = { version = "0.12", features = ["json", "rustls-tls", "stream"] }
  ```

- **Dependencies:** Task 5.1 (model registry)
- **Acceptance criteria:** Downloads model from HuggingFace with progress events. Cancellation stops download and removes partial file. Concurrent download attempts are rejected. Failed downloads clean up partial files.

#### Task 5.3: Add IPC commands for model management

- **Files to modify:**
  - `src-tauri/src/lib.rs` — add Tauri commands
  - `src-tauri/src/main.rs` — register commands
- **Implementation details:**

  ```rust
  #[tauri::command]
  async fn get_available_models(app: AppHandle) -> Result<Vec<whisper::models::ModelStatus>, String> {
      let models_dir = app.path().app_data_dir()
          .map_err(|e| e.to_string())?
          .join("models");
      Ok(whisper::models::list_models(&models_dir))
  }

  #[tauri::command]
  async fn download_model(app: AppHandle, model_id: String) -> Result<(), String> {
      let models_dir = app.path().app_data_dir()
          .map_err(|e| e.to_string())?
          .join("models");
      whisper::download::download_model(&app, &model_id, &models_dir)
          .await
          .map_err(|e| e.to_string())
  }

  #[tauri::command]
  fn cancel_download() -> Result<(), String> {
      whisper::download::cancel_download();
      Ok(())
  }

  #[tauri::command]
  async fn delete_model(app: AppHandle, model_id: String) -> Result<(), String> {
      let models_dir = app.path().app_data_dir()
          .map_err(|e| e.to_string())?
          .join("models");

      // If this model is currently loaded, unload it first
      {
          let engine = WHISPER_ENGINE.read().map_err(|e| e.to_string())?;
          if let Some(ref e) = *engine {
              if e.loaded_model_id() == model_id {
                  drop(engine);
                  unload_whisper();
              }
          }
      }

      whisper::models::delete_model(&models_dir, &model_id)
          .map_err(|e| e.to_string())
  }

  #[tauri::command]
  fn get_whisper_status() -> Result<WhisperStatusResponse, String> {
      let engine = WHISPER_ENGINE.read().map_err(|e| e.to_string())?;
      let settings = SETTINGS.read().map_err(|e| e.to_string())?;

      if settings.transcription_mode != TranscriptionMode::Local {
          return Ok(WhisperStatusResponse { state: "notActive".into(), loaded_model: None });
      }

      match engine.as_ref() {
          Some(e) => Ok(WhisperStatusResponse {
              state: "ready".into(),
              loaded_model: Some(e.loaded_model_id().to_string()),
          }),
          None => Ok(WhisperStatusResponse {
              state: "noModel".into(),
              loaded_model: None,
          }),
      }
  }
  ```

  Register in `main.rs` invoke handler:
  ```rust
  .invoke_handler(tauri::generate_handler![
      // existing commands...
      get_available_models,
      download_model,
      cancel_download,
      delete_model,
      get_whisper_status,
  ])
  ```

- **Dependencies:** Task 5.1, Task 5.2, Task 3.3
- **Acceptance criteria:** All five IPC commands callable from frontend. `download_model` emits `whisper-download-progress` events. `get_whisper_status` reflects current engine state.

#### Task 5.4: Add frontend command wrappers

- **Files to modify:**
  - `src/lib/commands.ts` — add TypeScript wrappers
- **Implementation details:**

  ```typescript
  export async function getAvailableModels(): Promise<WhisperModelInfo[]> {
    return invoke('get_available_models');
  }

  export async function downloadModel(modelId: string): Promise<void> {
    return invoke('download_model', { modelId });
  }

  export async function cancelDownload(): Promise<void> {
    return invoke('cancel_download');
  }

  export async function deleteModel(modelId: string): Promise<void> {
    return invoke('delete_model', { modelId });
  }

  export async function getWhisperStatus(): Promise<WhisperStatus> {
    return invoke('get_whisper_status');
  }
  ```

- **Dependencies:** Task 5.3 (IPC commands exist), Task 1.2 (types defined)
- **Acceptance criteria:** All wrappers compile and invoke the correct Tauri commands.

### Phase 6: Settings UI

#### Task 6.1: Create mode toggle component

- **Files to create:**
  - `src/components/ModeToggle.svelte`
- **Implementation details:**

  A segmented control or toggle switch with two options: "Local (Whisper)" and "Cloud (API)". Binds to `settings.transcriptionMode`. Emits change event to parent for conditional panel rendering.

  Uses the same styling patterns as existing settings components (Tailwind utility classes).

- **Dependencies:** Task 1.2 (types)
- **Acceptance criteria:** Toggle switches between `'local'` and `'cloud'`. Visually indicates active mode.

#### Task 6.2: Create Whisper configuration panel

- **Files to create:**
  - `src/components/WhisperConfig.svelte`
- **Implementation details:**

  Panel contains:
  - **Model list** (Req #29, #34, #35): Table/list showing all three tiers. Each row shows: name, description, size, download status. Downloaded models show file size and "Delete" button. Not-downloaded models show "Download" button.
  - **Download progress** (Req #24): Progress bar with percentage and MB count. Shows during active download. "Cancel" button (Req #25).
  - **Language selector** (Req #17): Dropdown with Auto/German/English. Bound to `settings.whisperConfig.language`.
  - **Status indicator** (Req #36): Badge showing "Model loaded and ready" (green), "No model downloaded" (yellow), "Loading model..." (blue).

  Listen to `whisper-download-progress` Tauri event for live progress updates:
  ```typescript
  import { listen } from '@tauri-apps/api/event';

  let downloadProgress: DownloadProgress | null = null;

  onMount(() => {
    const unlisten = listen<DownloadProgress>('whisper-download-progress', (event) => {
      downloadProgress = event.payload;
    });
    return () => { unlisten.then(fn => fn()); };
  });
  ```

  After download completes, refresh model list via `getAvailableModels()` and whisper status via `getWhisperStatus()`.

- **Dependencies:** Task 5.4 (command wrappers), Task 6.1 (mode toggle)
- **Acceptance criteria:** All model tiers displayed with correct download status. Download starts on button click with progress. Cancel stops download. Delete removes model. Language selector persists choice. Status badge reflects engine state.

#### Task 6.3: Integrate mode toggle and conditional panels into settings

- **Files to modify:**
  - `src/App.svelte` (or settings layout component) — add mode toggle, conditional rendering
  - `src/components/ProviderConfig.svelte` — no changes needed (wrapped conditionally by parent)
- **Implementation details:**

  At the top of settings, render `ModeToggle`. Below it:
  ```svelte
  {#if settings.transcriptionMode === 'local'}
    <WhisperConfig bind:settings />
  {:else}
    <ProviderConfig bind:settings />
  {/if}
  ```

  When mode changes (Req #41), call `save_settings()` to trigger backend `apply_settings()` which unloads Whisper if switching to Cloud.

  If switching to Local mode with no model downloaded (Req #22): WhisperConfig panel should prominently show the "no model" state with a prompt and "Download Now" button.

- **Dependencies:** Task 6.1, Task 6.2
- **Acceptance criteria:** Mode toggle is visible at top of settings. Switching modes shows the correct panel. Cloud panel preserves existing provider config. Settings persist on mode change.

## 6. Data Model Changes

No database changes. All persistence is via the existing JSON settings file (`~/.pisum-transcript.json`). New fields are added to `AppSettings` with `#[serde(default)]` for backward compatibility.

## 7. API Changes

No HTTP API changes. New Tauri IPC commands are defined in Task 5.3:

| Command | Parameters | Returns | Purpose |
|---------|------------|---------|---------|
| `get_available_models` | none | `Vec<ModelStatus>` | List model tiers with download status |
| `download_model` | `model_id: String` | `()` | Start model download (emits progress events) |
| `cancel_download` | none | `()` | Cancel active download |
| `delete_model` | `model_id: String` | `()` | Remove downloaded model file |
| `get_whisper_status` | none | `WhisperStatus` | Engine state (ready/loading/noModel/notActive) |

Event: `whisper-download-progress` — emitted during download with `DownloadProgress` payload.

## 8. Dependencies & Risks

### External Dependencies

| Dependency | Version | Purpose | Risk |
|------------|---------|---------|------|
| `whisper-rs` | 0.16 | Whisper.cpp Rust bindings | C++ compilation adds build time. Metal/Vulkan SDK required. |
| `futures-util` | 0.3 | Async stream processing for downloads | Widely used, low risk |
| HuggingFace CDN | — | Model file hosting | Network dependency for downloads only (not inference) |

### Technical Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| whisper-rs C++ build failures in CI | Build blocked | Test CI builds early in Phase 3. Vulkan SDK install step needed for Windows CI. |
| Large model files slow down dev iteration | Dev friction | Use `base` model (~60 MB) during development |
| WhisperContext is not `Send` | Threading issues | Hold context in `RwLock`, access only from the transcription thread. Verify whisper-rs thread safety. |
| GPU acceleration fails silently | Poor performance | Req #39 handles this — whisper.cpp falls back to CPU. Log a warning. |
| Reqwest streaming requires `stream` feature | Compile error | Already called out in Task 5.2 |

### Assumptions

- whisper-rs v0.16 API is stable and matches the code snippets above. Verify against actual crate docs before implementing.
- HuggingFace CDN allows direct file downloads without authentication for public repos.
- The app's Tauri bundle identifier in `tauri.conf.json` is `net.pisum.transcript` (confirmed from exploration).

## 9. Testing Strategy

### Unit Tests

| Area | Test Cases |
|------|-----------|
| `resample_for_whisper()` | Stereo→mono mixdown. 48kHz→16kHz resampling. 16kHz passthrough. Empty input. |
| `ModelTier` registry | `get_model_tier()` returns correct tier. Unknown ID returns `None`. |
| `verify_model()` | Valid file passes. Missing file fails. Wrong-size file fails. |
| `WhisperConfig` serde | Default deserialization. Round-trip serialization. |

### Integration Tests

| Scenario | How to Test |
|----------|-------------|
| Cloud mode transcription | Existing test path — unchanged. |
| Local mode transcription | Record short audio clip, process through full pipeline with `base` model. Verify non-empty English text returned. |
| Model download + verify | Download `base` model (~60 MB), verify file exists and passes integrity check. |
| Mode switching | Switch Local→Cloud, verify `WHISPER_ENGINE` is `None`. Switch back, transcribe, verify model reloads. |

### Manual Testing

| Scenario | Platforms |
|----------|-----------|
| Metal GPU acceleration | macOS Apple Silicon |
| Vulkan GPU acceleration | Windows with iGPU |
| CPU fallback | Any (disable GPU features to verify) |
| German→English translation | Both platforms |
| Download progress UI | Both platforms |
| Download cancellation | Both platforms |
| Settings persistence across app restart | Both platforms |

### Edge Cases

- Transcribe in Local mode with no model downloaded → clear error message (Req #5)
- Delete the currently-loaded model → engine unloads, status updates
- Cancel download mid-stream → partial file cleaned up (Req #30)
- Very short recording (<0.5s) → Whisper should still return result (may be empty)
- Model file corrupted after download → detected on load, user prompted to re-download (Req #38)

## 10. Requirement Traceability

### Functional Requirements

| PRD Ref | Requirement Summary | Task(s) | Notes |
|---------|-------------------|---------|-------|
| 4.1 #1 | Transcription mode setting | 1.1, 6.1 | |
| 4.1 #2 | Local mode bypasses cloud pool | 4.1 | |
| 4.1 #3 | Cloud mode uses existing pool | 4.1 | Unchanged path |
| 4.1 #4 | Persist transcription mode | 1.1 | Via `AppSettings` serde |
| 4.1 #5 | Prevent local transcription without model | 3.3, 6.2 | Error in `ensure_whisper_loaded()` + UI prompt |
| 4.2 #6 | Raw PCM for Whisper, encoded for cloud | 4.1 | Pipeline branching |
| 4.2 #7 | Skip encoding, resample to 16kHz mono | 2.1, 4.1 | |
| 4.2 #8 | Use existing rubato resampler | 2.1 | Same sinc params |
| 4.2 #9 | Mix down multi-channel to mono | 2.1 | Channel averaging |
| 4.2 #10 | Pass resampled samples to WhisperState::full() | 3.2 | |
| 4.3 #11 | Use whisper-rs v0.16 | 3.1 | |
| 4.3 #12 | Metal feature on macOS | 3.1 | Platform-conditional Cargo.toml |
| 4.3 #13 | Vulkan feature on Windows | 3.1 | Platform-conditional Cargo.toml |
| 4.3 #14 | Lazy load model, keep in memory | 3.3 | `ensure_whisper_loaded()` |
| 4.3 #15 | Configure translate task | 3.2 | `params.set_translate(true)` |
| 4.3 #16 | Translate task for English output | 3.2 | |
| 4.3 #17 | Configurable language hint | 1.1, 3.2, 6.2 | `WhisperLanguage` enum, language dropdown |
| 4.3 #18 | Concatenate text segments | 3.2 | Loop over segments |
| 4.3 #19 | No system prompt for Whisper | 4.1 | Local path doesn't call `active_system_prompt()` |
| 4.4 #20 | Three model tiers | 5.1 | `MODEL_TIERS` const |
| 4.4 #21 | Store in app data dir via Tauri 2 API | 5.1, 5.3 | `app.path().app_data_dir().join("models")` |
| 4.4 #22 | Prompt for download, don't auto-start | 6.2 | UI prompt with "Download Now" button |
| 4.4 #23 | Download from HuggingFace | 5.1, 5.2 | URLs in `MODEL_TIERS` |
| 4.4 #24 | Download progress indicator | 5.2, 6.2 | Tauri events + progress bar |
| 4.4 #25 | Cancel download | 5.2, 5.3, 6.2 | `cancel_download()` command |
| 4.4 #26 | Verify model integrity (file size) | 5.1, 5.2 | `verify_model()` after download |
| 4.4 #27 | Switch between downloaded models | 1.1, 6.2 | Model dropdown bound to `whisper_config.selected_model` |
| 4.4 #28 | Delete downloaded models | 5.1, 5.3, 6.2 | `delete_model()` command + UI button |
| 4.4 #29 | Display downloaded models with sizes | 5.1, 5.3, 6.2 | `list_models()` → model list UI |
| 4.4 #30 | Handle download failures, cleanup partials | 5.2 | Error handling + `remove_file` on failure |
| 4.4 #31 | One download at a time | 5.2 | `DOWNLOAD_IN_PROGRESS` atomic flag |
| 4.5 #32 | Mode toggle in settings UI | 6.1, 6.3 | |
| 4.5 #33 | Whisper config panel in Local mode | 6.2, 6.3 | |
| 4.5 #34 | Cloud config panel in Cloud mode | 6.3 | Conditional rendering |
| 4.5 #35 | Model dropdown shows download status | 6.2 | Per-model status badges |
| 4.5 #36 | Visual indicator for model ready state | 6.2 | Status badge component |
| 4.6 #37 | Error notification on inference failure | 4.1 | Existing `categorize_error()` handles `Transcription` |
| 4.6 #38 | Notify on missing/corrupt model, prompt re-download | 3.3, 6.2 | `ensure_whisper_loaded()` error + UI state |
| 4.6 #39 | GPU fallback to CPU, notify user | 3.2 | whisper.cpp handles fallback internally; log warning at info level since detection is not exposed by whisper-rs |
| 4.7 #40 | Default to Cloud mode | 1.1 | `TranscriptionMode::default() = Cloud` |
| 4.7 #41 | Unload Whisper on mode switch to Cloud | 3.3 | `apply_settings()` calls `unload_whisper()` |

### User Stories

| PRD Story | Summary | Implementing Tasks | Fully Covered? |
|-----------|---------|-------------------|----------------|
| US-1 | Privacy-conscious local transcription | 3.2, 4.1 | Yes |
| US-2 | Offline transcription | 3.2, 4.1 | Yes |
| US-3 | No API costs | 1.1, 4.1 | Yes |
| US-4 | GPU acceleration (macOS) | 3.1 | Yes (Metal feature) |
| US-5 | Vulkan acceleration (Windows) | 3.1 | Yes (Vulkan feature) |
| US-6 | Guided model download | 5.2, 5.3, 6.2 | Yes |
| US-7 | Delete unused models | 5.1, 5.3, 6.2 | Yes |
| US-8 | Switch to smaller model | 1.1, 6.2 | Yes |

### Success Metrics

| Metric | How the Plan Addresses It |
|--------|--------------------------|
| <3s latency on M4 (Metal) | Task 3.1 enables Metal. Task 3.2 uses greedy decoding. Manual test. |
| <5s latency on ThinkPad E14 (Vulkan) | Task 3.1 enables Vulkan. Manual test. |
| Semantic equivalence with F16 | Manual spot-check of 5 clips with `large-v3-turbo` quantized model. |
| Download with progress on both platforms | Task 5.2 (progress events), Task 6.2 (UI). Manual test. |
| Zero runtime crashes | All whisper-rs calls wrapped in `Result`. Error categorization in hotkey manager. |
| Startup time +2s max | Task 3.3: lazy loading (no model load on startup). Manual test. |
