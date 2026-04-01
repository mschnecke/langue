# PRD: Local Whisper Transcription Support

## 1. Introduction/Overview

Pisum Transcript currently relies exclusively on cloud AI providers (Gemini, OpenAI) for speech-to-text transcription. This feature adds offline, local transcription using whisper.cpp via the `whisper-rs` Rust crate. Users will be able to transcribe speech entirely on-device without an internet connection or API costs.

Whisper runs as a separate "local engine" mode distinct from the cloud provider pool. Users choose either local Whisper transcription or cloud-based transcription — they are not mixed in the round-robin provider pool.

## 2. Goals

- Enable fully offline speech-to-text transcription with no internet dependency
- Achieve sub-3-second transcription latency for 30-second audio clips on target hardware (Apple Silicon M4, Intel/AMD x86-64 with iGPU)
- Support German and English speech input with English-only text output
- Provide automatic model download and management with progress tracking
- Leverage platform-specific GPU acceleration (Metal on macOS, Vulkan on Windows)
- Maintain the existing cloud provider workflow as an alternative — users choose one mode or the other

## 3. User Stories

- As a privacy-conscious user, I want to transcribe my speech locally on my device so that my audio data never leaves my machine.
- As a user without reliable internet, I want offline transcription so that the app works anywhere regardless of connectivity.
- As a user who wants to avoid recurring costs, I want local transcription so that I don't need API keys or pay per-request fees.
- As a user on a MacBook with Apple Silicon, I want transcription to use my GPU so that results come back in under 3 seconds.
- As a user on a Windows laptop with integrated graphics, I want Vulkan acceleration so that transcription is fast without a dedicated GPU.
- As a first-time user, I want the app to guide me through downloading the required model so that I don't have to find and manage model files manually.
- As a user with limited disk space, I want to delete models I no longer need so that I can reclaim storage.
- As a user who needs faster transcription, I want to switch to a smaller model so that I get results more quickly at the cost of accuracy.

## 4. Functional Requirements

### 4.1 Transcription Mode Selection

1. The system must provide a setting to choose between "Local (Whisper)" and "Cloud" transcription modes.
2. When "Local (Whisper)" mode is selected, the system must bypass the cloud provider pool entirely and use the local Whisper engine for all transcription.
3. When "Cloud" mode is selected, the system must use the existing cloud provider pool (Gemini/OpenAI) as it does today.
4. The system must persist the selected transcription mode in settings.
5. The system must prevent transcription attempts in Local mode if no model is downloaded, and display a clear message directing the user to download a model.

### 4.2 Audio Pipeline

6. The system must pass raw f32 PCM samples (16 kHz, mono) to the Whisper provider and encoded audio bytes with MIME type to cloud providers. The audio pipeline must determine the correct format based on the active transcription mode before invoking the provider.
7. When the Whisper provider is active, the system must skip Opus/WAV encoding and instead resample the raw cpal f32 PCM audio to 16 kHz mono.
8. The system must use the existing `rubato` resampling infrastructure (already in the project for Opus encoding) to perform the 16 kHz resampling.
9. If the source audio is multi-channel, the system must mix down to mono before passing to Whisper.
10. The resampled 16 kHz mono f32 samples must be passed directly to `whisper_rs::WhisperState::full()`.

### 4.3 Whisper Engine Integration

11. The system must use `whisper-rs` v0.16.0 (or latest compatible) to perform local transcription.
12. On macOS, the system must compile with the `metal` feature flag to enable Apple Silicon GPU acceleration.
13. On Windows, the system must compile with the `vulkan` feature flag to enable integrated GPU acceleration.
14. The system must load the selected GGML model file into a `WhisperContext` on the first transcription request after app launch (or after model selection change) and keep it in memory for subsequent transcriptions.
15. The system must configure Whisper parameters for the **translate** task to ensure English output.
16. The system must use Whisper's **translate** task (not transcribe) to produce English text output regardless of the spoken language (German or English).
17. The system must allow the user to configure the input language hint (Auto, German, or English) to improve detection accuracy, but output is always English.
18. The system must extract all text segments from the Whisper result and concatenate them into the final transcription output.
19. The system must NOT use the preset system prompt for Whisper — system prompts are cloud-provider only. Whisper uses its own language/task configuration.

### 4.4 Model Management

20. The system must support three model tiers:
    - Large: `ggml-large-v3-turbo-q5_0.bin` (~574 MB) — default, best accuracy
    - Small: `ggml-small-q5_1.bin` (~200 MB) — lightweight alternative
    - Base: `ggml-base-q5_1.bin` (~60 MB) — minimal, for constrained hardware
21. The system must store downloaded models in the platform-appropriate app data directory using Tauri 2's `AppHandle::path().app_data_dir()` API, under a `models/` subdirectory.
22. When Local mode is selected and no model is downloaded, the system must display a prompt explaining that a model download is required (~574 MB for the default model) and provide a 'Download Now' button. The download must not start automatically.
23. The system must download models from HuggingFace's `ggerganov/whisper.cpp` repository (the canonical source for GGML Whisper models).
24. The system must display a download progress indicator (percentage and MB downloaded) in the settings UI during model downloads.
25. The system must allow the user to cancel an in-progress model download.
26. The system must verify downloaded model file integrity (file size check at minimum).
27. The system must allow the user to switch between downloaded models in the settings UI.
28. The system must allow the user to delete downloaded models to reclaim disk space.
29. The system must display which models are currently downloaded and their file sizes in the settings UI.
30. The system must handle download failures gracefully — including network timeouts, DNS resolution failures, and incomplete downloads — by displaying a specific error message describing the failure and allowing retry. Partially downloaded files must be cleaned up on failure.
31. The system must support only one model download at a time. Initiating a new download while one is in progress must be prevented, with a message indicating the current download must complete or be cancelled first.

### 4.5 Settings UI

32. The settings UI must include a new top-level toggle or tab for selecting the transcription mode (Local vs. Cloud).
33. When Local mode is selected, the settings UI must show Whisper-specific configuration:
    - Model selection (dropdown of available models with download status)
    - Language preference (Auto / German / English)
    - Download/delete model buttons
    - Download progress indicator
    - Current model status (loaded, not downloaded, downloading)
34. When Cloud mode is selected, the settings UI must show the existing provider configuration (unchanged).
35. The model selection dropdown must indicate which models are downloaded (with file size) vs. available for download.
36. The settings UI must display a visual indicator when the Whisper model is loaded and ready for transcription.

### 4.6 Error Handling

37. If Whisper inference fails, the system must display an error notification via the system tray (consistent with existing cloud provider error behavior).
38. If the selected model file is missing or corrupted, the system must notify the user and prompt re-download.
39. If GPU acceleration fails at runtime, the system must fall back to CPU-only inference transparently and notify the user.

### 4.7 Mode Switching

40. New installations must default to Cloud transcription mode.
41. When the user switches from Local to Cloud mode, the system must unload the WhisperContext to free memory. When switching back to Local mode, the model is reloaded on the next transcription request (per Req #14).

## 5. Non-Goals (Out of Scope)

- Not included: Mixing Whisper and cloud providers in the same round-robin pool. Users pick one mode.
- Not included: Real-time / streaming transcription. Whisper processes complete audio clips after recording stops (same as current cloud behavior).
- Not included: Configurable output language. Whisper always outputs English text (using the translate task for non-English input).
- Not included: Custom or user-provided GGML model files. Only the three predefined model tiers are supported.
- Not included: Whisper prompt/initial_prompt configuration. Whisper operates with its default prompting behavior.
- Not included: NVIDIA Parakeet/Canary model support (potential future enhancement via `transcribe-rs` crate).
- Not included: CoreML/ANE acceleration on macOS. Metal GPU acceleration is sufficient for the target use case.
- Not included: Linux platform support. Linux builds are not part of the initial Whisper integration. GPU acceleration backend (CUDA, Vulkan, or CPU-only) for Linux will be evaluated separately.
- Not included: Model bundling in the installer. Models are downloaded on-demand to keep the installer small.
- Not included: Cross-compilation. Each platform is built natively in CI.

## 6. Design Considerations

### Settings UI Layout

The settings page gains a new top-level control for transcription mode. When "Local (Whisper)" is selected, the Providers tab is replaced with a Whisper configuration panel.

**Whisper Configuration Panel:**

- Model selector showing three tiers with descriptions (accuracy vs. speed vs. size tradeoffs)
- Download status per model: "Not downloaded", "Downloading (45%)", "Downloaded (574 MB)"
- Download/Delete action buttons per model
- Input language hint dropdown: Auto (default), German, English (output is always English)
- Status indicator: "Model loaded and ready" / "No model downloaded" / "Loading model..."

**Mode Toggle:**

- Prominent toggle at the top of settings: "Local (Whisper)" | "Cloud (API)"
- Switching modes immediately changes which configuration panel is visible
- The inactive mode's configuration is preserved but hidden

## 7. Technical Considerations

### Dependencies

- `whisper-rs` v0.16.0 with platform-conditional features:

  ```toml
  [target.'cfg(target_os = "macos")'.dependencies]
  whisper-rs = { version = "0.16", features = ["metal"] }

  [target.'cfg(target_os = "windows")'.dependencies]
  whisper-rs = { version = "0.16", features = ["vulkan"] }
  ```

- Existing `rubato` crate is reused for 16 kHz resampling

### Trait Modification

The `TranscriptionProvider::transcribe()` method must accept an `AudioInput` enum with variants `Encoded { data: Vec<u8>, mime_type: String }` and `Raw { samples: Vec<f32>, sample_rate: u32 }`. The pipeline selects the appropriate variant based on the active transcription mode.

### Model Storage

Models are stored under the `models/` subdirectory of the platform app data directory, resolved via Tauri 2's `AppHandle::path().app_data_dir()` API using the bundle identifier from `tauri.conf.json`. The exact paths are platform-determined and should not be hardcoded.

### WhisperContext Lifecycle

The `WhisperContext` (loaded model) should be held in a global state similar to `PROVIDER_POOL`. Loading a model takes 1-5 seconds depending on model size and hardware, so it should be loaded lazily on the first transcription request (not eagerly on startup). The context must be unloaded when switching to Cloud mode to free memory (see Req #41).

### Build System Impact

whisper.cpp compiles from C/C++ source via `whisper-rs-sys`. This adds:

- ~30-60 seconds to clean build time
- Requires C/C++ toolchain on the build machine (already required for Tauri)
- Metal SDK on macOS (included with Xcode CLI tools)
- Vulkan SDK on Windows (needs to be installed in CI)

### Tauri IPC Commands

New Tauri commands needed:

- `get_available_models()` — returns list of model tiers with download status
- `download_model(model_id)` — initiates download, emits progress events
- `cancel_download()` — cancels in-progress download
- `delete_model(model_id)` — removes downloaded model file
- `get_whisper_status()` — returns current engine state (ready, loading, no model)

Download progress should be communicated to the frontend via Tauri events (not polling).

### Reference Implementations

- **Handy** (github.com/cjpais/Handy) — Tauri 2 + whisper-rs + cpal, MIT licensed. Reference for model management and whisper-rs integration.
- **Whispering** (github.com/braden-w/whispering) — Svelte 5 + Tauri + whisper-rs. Reference for frontend architecture. AGPLv3 (do not copy code).

## 8. Success Metrics

- Transcription latency under 3 seconds for 30-second audio clips on Apple Silicon M4 with Metal acceleration
- Transcription latency under 5 seconds for 30-second audio clips on ThinkPad E14 x86-64 with Vulkan acceleration
- Quantized model output for a 30-second German-to-English test clip must produce semantically equivalent text to the same clip processed by the F16 model (manual spot-check of 5 representative clips)
- Model download completes successfully with progress tracking on both macOS and Windows
- Zero runtime crashes from whisper-rs integration on either platform
- App startup time increases by no more than 2 seconds when a Whisper model is loaded

## 9. Open Questions

> All open questions have been resolved.

- [x] Should the app support downloading models from a custom/mirror URL for users behind corporate firewalls? -> No
- [x] What is the fallback behavior if Vulkan drivers are not available on a Windows machine — should the app auto-detect and disable the Vulkan feature, or let whisper.cpp fall back to CPU transparently? -> Let whisper.cpp fall back to CPU transparently.
- [x] Should model downloads resume after interruption (HTTP range requests), or restart from scratch? -> Restart from scratch.
- [x] Is there a minimum disk space check needed before initiating a 574 MB model download? -> No, the model download is triggered by the user clicking the "Download" button.
- [x] Should the app pre-warm the Whisper model (load into GPU memory) on startup, or load on first transcription request? -> Load on first transcription request.
- [x] How should the settings schema version be handled — is a migration needed for existing users' settings files when adding the new transcription mode fields? -> No, the settings schema is versioned independently of the app.
