# PRD: OpenAI as Transcription Provider

## Overview

Add OpenAI as a second AI transcription provider alongside Gemini. OpenAI's Chat Completions API accepts audio input via `input_audio` content blocks with full system prompt support, enabling the same workflow as Gemini: send audio + system prompt, receive transcribed and formatted text. This gives users provider choice, redundancy through round-robin failover, and access to OpenAI's speech-to-text capabilities with preset support.

## Problem Statement

Currently, Pisum Transcript supports only Gemini as a transcription provider. Users who prefer OpenAI, want provider redundancy, or experience Gemini rate limits have no alternative. The provider abstraction (`TranscriptionProvider` trait) and round-robin pool already exist but only have one implementation.

## Goals & Success Metrics

| Goal | Metric |
|------|--------|
| OpenAI transcribes audio with same quality as Gemini | Successful transcription of Opus and WAV audio clips |
| Full preset compatibility | All existing system prompts produce expected output via Chat Completions |
| Provider failover works | Pool falls back to Gemini if OpenAI fails (and vice versa) |
| Model selection works | Only audio-capable OpenAI models shown in UI |
| Seamless UX | Adding OpenAI feels identical to adding Gemini in settings |

## User Stories

1. **As a user**, I want to add my OpenAI API key and select a model, so I can use OpenAI for transcription.
2. **As a user**, I want to see only OpenAI models that support audio input, so I don't accidentally pick an incompatible model.
3. **As a user**, I want my existing presets (system prompts) to work with OpenAI, so I don't need to duplicate them per provider.
4. **As a user**, I want to enable both Gemini and OpenAI providers simultaneously, so the app automatically fails over between them.
5. **As a user**, I want to test my OpenAI API key from the settings UI before using it.

## Functional Requirements

### FR-1: OpenAI Provider Implementation (Rust)

Create `src-tauri/src/ai/openai.rs` implementing `TranscriptionProvider`:

- **API endpoint**: `https://api.openai.com/v1/chat/completions`
- **Authentication**: `Authorization: Bearer <api_key>` header
- **Request format**: JSON with base64-encoded audio (same pattern as Gemini)
- **Default model**: `gpt-4o-mini-audio-preview`
- **Temperature**: 0.1 (matching Gemini for deterministic output)
- **Max tokens**: 8192 (matching Gemini)

**Request structure:**
```json
{
  "model": "gpt-4o-mini-audio-preview",
  "temperature": 0.1,
  "max_tokens": 8192,
  "messages": [
    {
      "role": "system",
      "content": "<system_prompt from active preset>"
    },
    {
      "role": "user",
      "content": [
        {
          "type": "input_audio",
          "input_audio": {
            "data": "<base64_encoded_audio>",
            "format": "wav"
          }
        }
      ]
    }
  ]
}
```

**Audio format mapping:**
- `audio/wav` → `"format": "wav"`
- `audio/ogg` (Opus) → `"format": "ogg"` (verify this is accepted; fallback to WAV if not)

**Key similarities with Gemini:**
- JSON request body with base64-encoded audio
- System prompt passed as a separate field (Gemini: `system_instruction`, OpenAI: `system` role message)
- Same temperature and max token settings
- Presets (system prompts) fully supported

**Response structure:**
```json
{
  "choices": [
    {
      "message": {
        "role": "assistant",
        "content": "<transcribed text>"
      }
    }
  ]
}
```

**Error response structure:**
```json
{
  "error": {
    "message": "...",
    "type": "...",
    "code": "..."
  }
}
```

**Retry logic:** Same pattern as Gemini — 3 attempts with exponential backoff (1s, 2s, 3s). Retryable on HTTP 429 (rate limit) and 503 (service unavailable).

### FR-2: Connection Test

- **Endpoint**: `GET https://api.openai.com/v1/models`
- **Authentication**: `Authorization: Bearer <api_key>`
- Send a lightweight GET request to list models. If it returns HTTP 200, the API key is valid.
- No need to send audio — this is faster and cheaper than a test transcription.

### FR-3: Model Listing (Audio-Capable Only)

- **Endpoint**: `GET https://api.openai.com/v1/models`
- **Authentication**: `Authorization: Bearer <api_key>`
- **Response**: Returns a list of all models the API key has access to
- **Filter**: Client-side filter to only show audio-capable chat models. Match model IDs against known audio-capable prefixes:
  - `gpt-4o-audio-preview` (and dated variants)
  - `gpt-4o-mini-audio-preview` (and dated variants)
- **Note**: Do NOT include `whisper-1`, `gpt-4o-transcribe`, or `gpt-4o-mini-transcribe` — those are for the dedicated transcription endpoint which doesn't support system prompts.
- **Fallback**: If the API call fails, provide a hardcoded list of known audio-capable models.

### FR-4: Configuration Schema Update

Add `OpenAi` variant to `ProviderType` enum in `src-tauri/src/config/schema.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    Gemini,
    #[serde(rename = "openai")]
    OpenAi,
}
```

No changes needed to `ProviderConfig` struct — `id`, `provider_type`, `api_key`, `model`, `enabled` fields are sufficient.

### FR-5: Provider Pool Integration

Update `src-tauri/src/ai/pool.rs` to:
- Match `"openai"` in `rebuild()` to create `OpenAiProvider` instances
- Match `"openai"` in `test_provider()` to test OpenAI connections
- No changes to round-robin or failover logic needed

### FR-6: IPC Command Updates

Update `src-tauri/src/lib.rs`:
- `list_provider_models`: Add `"openai"` match to call `OpenAiProvider::list_models()`
- `test_provider_connection`: Add `OpenAi` variant to the `ProviderType` match in both `test_provider_connection` and `apply_settings`

### FR-7: Frontend Settings UI

Update `src/components/ProviderConfig.svelte`:
- Add "OpenAI" to provider type dropdown (alongside "Gemini")
- Model dropdown should fetch and display audio-capable OpenAI models
- Default model shown when no model is selected: `gpt-4o-mini-audio-preview`
- Default option label should be provider-specific: `Default (gpt-4o-mini-audio-preview)` for OpenAI vs `Default (gemini-2.5-flash-lite)` for Gemini
- Test connection button works with OpenAI API key
- No preset limitation note needed — system prompts are fully supported via Chat Completions

Update `src/lib/types.ts`:
- Add `'openai'` to `providerType` union type

## Non-Functional Requirements

### NFR-1: Performance
- Connection test should complete within 10 seconds
- Transcription latency comparable to Gemini (network-bound)

### NFR-2: Error Handling
- Clear error messages distinguishing authentication failures from rate limits
- Graceful fallback when OpenAI is unavailable (pool handles this)
- Handle OpenAI-specific error response format: `{ "error": { "message": "...", "type": "...", "code": "..." } }`

### NFR-3: Security
- API key stored in local config file only (same as Gemini)
- API key never logged
- API key sent via `Authorization` header (standard Bearer token — more secure than Gemini's URL parameter approach)

### NFR-4: Backward Compatibility
- Existing Gemini-only configs must load without error
- `ProviderType` deserialization must handle `"gemini"` as before
- No migration needed — new `"openai"` variant is additive

## Technical Considerations

### Files to Create
| File | Purpose |
|------|---------|
| `src-tauri/src/ai/openai.rs` | OpenAI provider implementation + model listing |

### Files to Modify
| File | Change |
|------|--------|
| `src-tauri/src/ai/mod.rs` | Add `pub mod openai;` |
| `src-tauri/src/ai/pool.rs` | Add OpenAI match in `rebuild()` and `test_provider()` |
| `src-tauri/src/config/schema.rs` | Add `OpenAi` to `ProviderType` enum |
| `src-tauri/src/lib.rs` | Add OpenAI match in `list_provider_models`, `test_provider_connection`, and `apply_settings` |
| `src/components/ProviderConfig.svelte` | Add OpenAI to provider type dropdown, provider-specific default model label |
| `src/lib/types.ts` | Add `'openai'` to provider type |

### Dependencies
- No new Rust crates needed — `reqwest`, `serde`, `base64`, `tokio` already in use
- Same JSON + base64 pattern as Gemini — no multipart needed

### Audio Format Compatibility
- Reuse the user's configured audio format (Opus or WAV)
- Audio sent as base64 in `input_audio` content block
- `format` field in request: `"wav"` for WAV, `"ogg"` for Opus (verify Opus/OGG support; if not supported, encode as WAV before sending)

## Out of Scope

- Dedicated transcription endpoint (`/v1/audio/transcriptions`) — using Chat Completions for system prompt support
- Streaming transcription responses
- Speaker diarization
- Translation endpoint (`/v1/audio/translations`)
- Timestamp/word-level granularity
- Usage tracking or cost estimation
- Real-time WebSocket transcription

## Open Questions

1. **OGG/Opus format support in Chat Completions**: The `input_audio.format` field accepts `"wav"` and `"mp3"`. Verify whether `"ogg"` is also accepted. If not, when the user's audio format is Opus, we may need to always send WAV instead (the encoder already supports both formats with fallback). -> use MP3
2. **Audio-capable model discovery**: The OpenAI `/v1/models` endpoint doesn't expose capability metadata. Verify the exact model IDs that support `input_audio` in Chat Completions (currently believed to be `gpt-4o-audio-preview` and `gpt-4o-mini-audio-preview` families).

## References

- [OpenAI Audio and Speech Guide](https://platform.openai.com/docs/guides/audio)
- [OpenAI Chat Completions API](https://platform.openai.com/docs/api-reference/chat/create)
- [OpenAI Audio Models](https://platform.openai.com/docs/models/gpt-4o-audio-preview)
- [OpenAI List Models API](https://platform.openai.com/docs/api-reference/models/list)
- Existing Gemini implementation: `src-tauri/src/ai/gemini.rs`
- Provider trait: `src-tauri/src/ai/provider.rs`
- Provider pool: `src-tauri/src/ai/pool.rs`
