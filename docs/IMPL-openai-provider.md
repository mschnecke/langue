# Implementation Plan: OpenAI as Transcription Provider

> Generated from: `docs/PRD-openai-provider.md`
> Date: 2026-03-15

## 1. Overview

Add OpenAI as a second AI transcription provider alongside Gemini, using OpenAI's Chat Completions API with `input_audio` content blocks. The existing provider abstraction (`TranscriptionProvider` trait) and round-robin pool already support multiple providers — this plan fills the second slot.

The implementation closely mirrors the Gemini provider: same trait, same base64 audio encoding, same retry logic, same config structure. The main differences are the API endpoint, auth mechanism (Bearer token vs URL param), and request/response JSON shapes.

## 2. Architecture & Design

### Data Flow

```
User speaks → Audio recorded → Base64 encoded
                                    ↓
                          ProviderPool::transcribe()
                                    ↓
                          Round-robin selects provider
                          ┌─────────┴─────────┐
                     GeminiProvider      OpenAiProvider
                          │                    │
                   Gemini API            Chat Completions API
                   (generateContent)     (/v1/chat/completions)
                          └─────────┬─────────┘
                                    ↓
                          Transcribed text → Clipboard → Paste
```

### Integration Points

- **Provider trait**: `OpenAiProvider` implements `TranscriptionProvider` (same as `GeminiProvider`)
- **Pool**: `rebuild()` matches `"openai"` to create `OpenAiProvider` instances
- **Config**: `ProviderType::OpenAi` variant added to enum
- **IPC**: `list_provider_models` and `test_provider_connection` gain `"openai"` branches
- **Frontend**: Provider type dropdown gains "OpenAI" option; model dropdown fetches OpenAI models

### API Contracts

**Transcription request** — `POST https://api.openai.com/v1/chat/completions`
- Auth: `Authorization: Bearer <api_key>`
- Body: JSON with system message + user message containing `input_audio` block
- Response: `choices[0].message.content`

**Model listing** — `GET https://api.openai.com/v1/models`
- Auth: `Authorization: Bearer <api_key>`
- Response: `data[]` array of model objects, filtered client-side for audio-capable models

**Connection test** — Same `GET /v1/models` endpoint; HTTP 200 = valid key.

## 3. Phases & Milestones

### Phase 1: Backend — OpenAI Provider
**Goal:** Rust implementation of OpenAI transcription, model listing, and connection test
**Deliverable:** `OpenAiProvider` fully implements `TranscriptionProvider`; pool and IPC wired up

### Phase 2: Frontend — Settings UI
**Goal:** Users can add, configure, and test OpenAI providers from the settings UI
**Deliverable:** Provider type dropdown includes OpenAI; model listing and connection test work end-to-end

## 4. Files Overview

### Files to Create
| File Path | Purpose |
|-----------|---------|
| `src-tauri/src/ai/openai.rs` | OpenAI provider: transcription, model listing, connection test |

### Files to Modify
| File Path | What Changes |
|-----------|-------------|
| `src-tauri/src/ai/mod.rs` | Add `pub mod openai;` |
| `src-tauri/src/ai/pool.rs` | Add `"openai"` match arms in `rebuild()` and `test_provider()` |
| `src-tauri/src/config/schema.rs` | Add `OpenAi` variant to `ProviderType` enum |
| `src-tauri/src/lib.rs` | Add `"openai"` match in `list_provider_models`, `test_provider_connection`, and `apply_settings` |
| `src/lib/types.ts` | Add `'openai'` to `providerType` union |
| `src/components/ProviderConfig.svelte` | Add "OpenAI" to dropdown; provider-specific default model label |

## 5. Task Breakdown

### Phase 1: Backend — OpenAI Provider

#### Task 1.1: Add `OpenAi` to `ProviderType` enum
- **Files to modify:**
  - `src-tauri/src/config/schema.rs` — Add `OpenAi` variant with `#[serde(rename = "openai")]`
- **Implementation details:**
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
  #[serde(rename_all = "lowercase")]
  pub enum ProviderType {
      Gemini,
      #[serde(rename = "openai")]
      OpenAi,
  }
  ```
- **Dependencies:** None
- **Acceptance criteria:** Existing `"gemini"` configs deserialize correctly; `"openai"` also deserializes

#### Task 1.2: Implement `OpenAiProvider` struct and `TranscriptionProvider` trait
- **Files to create:**
  - `src-tauri/src/ai/openai.rs` — Full provider implementation
- **Files to modify:**
  - `src-tauri/src/ai/mod.rs` — Add `pub mod openai;`
- **Implementation details:**

  **Struct:**
  ```rust
  pub struct OpenAiProvider {
      api_key: String,
      model: String,
      client: reqwest::Client,
  }

  impl OpenAiProvider {
      pub fn new(api_key: &str, model: Option<&str>) -> Self {
          Self {
              api_key: api_key.to_string(),
              model: model.unwrap_or("gpt-4o-mini-audio-preview").to_string(),
              client: reqwest::Client::new(),
          }
      }
  }
  ```

  **Transcription (`transcribe`):**
  - Base64-encode audio data (reuse `base64::engine::general_purpose::STANDARD`)
  - Map MIME type to format: `"audio/wav"` → `"wav"`, `"audio/ogg"` → `"mp3"` (per PRD open question resolution: use MP3 for Opus)
  - Build Chat Completions JSON request:
    ```rust
    json!({
        "model": self.model,
        "temperature": 0.1,
        "max_tokens": 8192,
        "messages": [
            { "role": "system", "content": system_prompt },
            {
                "role": "user",
                "content": [{
                    "type": "input_audio",
                    "input_audio": {
                        "data": base64_audio,
                        "format": audio_format
                    }
                }]
            }
        ]
    })
    ```
  - POST to `https://api.openai.com/v1/chat/completions`
  - Header: `Authorization: Bearer {api_key}`
  - Parse response: extract `choices[0].message.content`
  - Handle error responses: parse `error.message` from OpenAI error JSON
  - Retry logic: 3 attempts, exponential backoff (1s, 2s, 3s), retry on 429 and 503 (same pattern as `gemini.rs`)

  **Connection test (`test_connection`):**
  - GET `https://api.openai.com/v1/models` with Bearer auth
  - Return `Ok(true)` on 200, `Ok(false)` or error otherwise

  **Provider name:** Return `"OpenAI"`

- **Dependencies:** Task 1.1 (schema update)
- **Acceptance criteria:** Provider compiles; transcribe sends correct JSON structure; test_connection validates API key; retry logic handles 429/503

#### Task 1.3: Implement `list_models` static method
- **Files to modify:**
  - `src-tauri/src/ai/openai.rs` — Add `list_models()` function
- **Implementation details:**
  ```rust
  pub async fn list_models(api_key: &str) -> Result<Vec<ModelInfo>, AppError> {
      // GET https://api.openai.com/v1/models
      // Auth: Bearer {api_key}
      // Filter: keep only models whose ID starts with
      //   "gpt-4o-audio-preview" or "gpt-4o-mini-audio-preview"
      // Map to ModelInfo { id, display_name }
      // Fallback: if API call fails, return hardcoded list:
      //   [("gpt-4o-audio-preview", "GPT-4o Audio Preview"),
      //    ("gpt-4o-mini-audio-preview", "GPT-4o Mini Audio Preview")]
  }
  ```
  - Response JSON shape: `{ "data": [{ "id": "model-id", "owned_by": "..." }, ...] }`
  - Display name: humanize model ID (e.g., `"gpt-4o-audio-preview"` → `"GPT-4o Audio Preview"`)
  - Explicitly exclude `whisper-1`, `gpt-4o-transcribe`, `gpt-4o-mini-transcribe`
- **Dependencies:** Task 1.2
- **Acceptance criteria:** Returns only audio-capable chat models; falls back to hardcoded list on API failure

#### Task 1.4: Wire OpenAI into provider pool and IPC commands
- **Files to modify:**
  - `src-tauri/src/ai/pool.rs` — Add OpenAI match in `rebuild()` and `test_provider()`
  - `src-tauri/src/lib.rs` — Add `"openai"` match in `list_provider_models`, `test_provider_connection`, and `apply_settings`
- **Implementation details:**

  **pool.rs — `rebuild()`:**
  ```rust
  // In the match on provider_type:
  "openai" => {
      providers.push(Box::new(OpenAiProvider::new(
          &entry.api_key,
          entry.model.as_deref(),
      )));
  }
  ```

  **pool.rs — `test_provider()`:**
  ```rust
  "openai" => {
      let provider = OpenAiProvider::new(&entry.api_key, entry.model.as_deref());
      provider.test_connection().await
  }
  ```

  **lib.rs — `list_provider_models`:**
  ```rust
  "openai" => openai::OpenAiProvider::list_models(&api_key).await,
  ```

  **lib.rs — `test_provider_connection`:**
  Add `ProviderType::OpenAi` to the match that builds a `ProviderEntry`, setting `provider_type: "openai".to_string()`.

  **lib.rs — `apply_settings`:**
  Add `ProviderType::OpenAi` arm in the match that maps `ProviderConfig` to `ProviderEntry`, mapping to `provider_type: "openai".to_string()`.

- **Dependencies:** Tasks 1.2, 1.3
- **Acceptance criteria:** Pool creates OpenAI providers from config; round-robin and failover work across Gemini + OpenAI; model listing and connection test callable via IPC

### Phase 2: Frontend — Settings UI

#### Task 2.1: Add `'openai'` to TypeScript types
- **Files to modify:**
  - `src/lib/types.ts` — Add `'openai'` to `providerType` union
- **Implementation details:**
  ```typescript
  providerType: 'gemini' | 'openai';
  ```
- **Dependencies:** None
- **Acceptance criteria:** TypeScript compiles with new union type

#### Task 2.2: Update `ProviderConfig.svelte` for OpenAI support
- **Files to modify:**
  - `src/components/ProviderConfig.svelte` — Provider type dropdown and default model label
- **Implementation details:**
  - Add "OpenAI" option to the provider type `<select>` dropdown (value: `"openai"`, label: `"OpenAI"`)
  - Update default model placeholder/label to be provider-specific:
    - Gemini: `Default (gemini-2.5-flash-lite)`
    - OpenAI: `Default (gpt-4o-mini-audio-preview)`
  - When provider type changes, clear model selection and cached models for that provider (trigger re-fetch)
  - When creating a new provider with type "openai", set `providerType: 'openai'` in the new `ProviderConfig` object
  - Model dropdown, test connection, and enable/disable toggle already work generically — no changes needed for those
- **Dependencies:** Task 2.1, Phase 1 complete
- **Acceptance criteria:** Can add an OpenAI provider from UI; model dropdown shows audio-capable models; test connection validates OpenAI API key; default model label shows correct provider-specific default

## 6. Data Model Changes

No database changes. The configuration schema change is limited to adding `OpenAi` to the `ProviderType` enum in `schema.rs`. The existing `ProviderConfig` struct fields (`id`, `provider_type`, `api_key`, `model`, `enabled`) are sufficient — no new fields needed.

Backward compatibility: existing JSON configs with only `"gemini"` providers deserialize without error since `OpenAi` is an additive variant.

## 7. API Changes

No new Tauri IPC commands. Existing commands gain OpenAI support via additional match arms:

| Command | Change |
|---------|--------|
| `list_provider_models` | New `"openai"` match calls `OpenAiProvider::list_models()` |
| `test_provider_connection` | New `ProviderType::OpenAi` match creates OpenAI `ProviderEntry` |
| `save_settings` → `apply_settings` | New `ProviderType::OpenAi` match in pool rebuild loop |

## 8. Dependencies & Risks

### External Dependencies
- **OpenAI Chat Completions API** — `input_audio` content blocks for audio transcription
- **OpenAI Models API** — `/v1/models` for listing and connection testing
- No new Rust crates — `reqwest`, `serde`, `serde_json`, `base64`, `tokio` already in `Cargo.toml`

### Risks & Mitigations
| Risk | Mitigation |
|------|------------|
| Opus/OGG not accepted by `input_audio.format` | PRD resolves: use `"mp3"` format for Opus audio |
| Audio-capable model IDs change over time | Hardcoded fallback list ensures models always available; prefix matching catches dated variants |
| OpenAI rate limits during transcription | Retry logic with backoff (same as Gemini); pool failover to Gemini |
| API key validation gives false positive (valid key but no audio model access) | Connection test only checks key validity; actual transcription errors handled by pool failover |

### Assumptions
- `input_audio` content blocks in Chat Completions accept `"wav"` and `"mp3"` formats
- Audio-capable models match prefixes `gpt-4o-audio-preview` and `gpt-4o-mini-audio-preview`
- The `/v1/models` endpoint is sufficient for connection testing (no audio-specific validation)

## 9. Testing Strategy

### Manual Testing
- **Connection test**: Enter valid/invalid OpenAI API key → verify success/failure message
- **Model listing**: Verify only `gpt-4o-audio-preview` and `gpt-4o-mini-audio-preview` families shown
- **Transcription**: Record audio → verify OpenAI returns correct transcription
- **Failover**: Disable Gemini, enable only OpenAI → verify transcription works; enable both → verify round-robin
- **Preset compatibility**: Use German transcription and English translation presets with OpenAI
- **Config persistence**: Add OpenAI provider, restart app, verify config loads correctly
- **Backward compatibility**: Load config with only Gemini providers, verify no errors

### Edge Cases
- Empty API key → clear error message
- Invalid API key → "authentication failed" error (not generic error)
- Rate limited (429) → retry with backoff, then failover
- Model listing API failure → hardcoded fallback list shown
- Opus audio format → sent as `"mp3"` format to OpenAI
- No model selected → defaults to `gpt-4o-mini-audio-preview`

## 10. Requirement Traceability

### Functional Requirements

| PRD Ref | Requirement Summary | Task(s) | Notes |
|---------|-------------------|---------|-------|
| FR-1 | OpenAI provider implementing `TranscriptionProvider` | 1.2 | Full trait implementation with retry logic |
| FR-2 | Connection test via `/v1/models` | 1.2 | Implemented in `test_connection()` |
| FR-3 | Model listing with audio-capable filter | 1.3 | Prefix matching + hardcoded fallback |
| FR-4 | `OpenAi` variant in `ProviderType` enum | 1.1 | With `#[serde(rename = "openai")]` |
| FR-5 | Provider pool integration | 1.4 | `rebuild()` and `test_provider()` match arms |
| FR-6 | IPC command updates | 1.4 | `list_provider_models`, `test_provider_connection`, `apply_settings` |
| FR-7 | Frontend settings UI | 2.1, 2.2 | Type update + dropdown + default model label |

### User Stories

| PRD Ref | User Story Summary | Implementing Tasks | Fully Covered? |
|---------|-------------------|-------------------|----------------|
| US-1 | Add OpenAI API key and select model | 1.1, 1.2, 1.4, 2.1, 2.2 | Yes |
| US-2 | See only audio-capable models | 1.3, 2.2 | Yes |
| US-3 | Existing presets work with OpenAI | 1.2 | Yes — system prompt passed as `system` role message |
| US-4 | Enable both providers for failover | 1.4 | Yes — pool round-robin unchanged |
| US-5 | Test API key from settings | 1.2, 1.4, 2.2 | Yes — connection test via `/v1/models` |

### Non-Functional Requirements

| PRD Ref | Requirement | Addressed By |
|---------|-------------|-------------|
| NFR-1 | Connection test < 10s; transcription latency comparable to Gemini | Lightweight GET for test; network-bound transcription |
| NFR-2 | Clear error messages; graceful fallback | OpenAI error parsing (`error.message`); pool failover |
| NFR-3 | API key security | Stored in local config only; Bearer header (not URL param); never logged |
| NFR-4 | Backward compatibility | Additive enum variant; no migration needed |

### Success Metrics

| Metric | How the Plan Addresses It |
|--------|--------------------------|
| Successful transcription of Opus and WAV audio | Task 1.2: transcribe() handles both formats (WAV → "wav", Opus → "mp3") |
| All existing system prompts work via Chat Completions | Task 1.2: system prompt sent as `system` role message — full support |
| Pool failover between providers | Task 1.4: pool.rs gains OpenAI match; existing round-robin/fallback logic handles rest |
| Only audio-capable models shown | Task 1.3: prefix-based filter on model IDs |
| Adding OpenAI feels identical to Gemini in settings | Task 2.2: same UI pattern — dropdown, key input, model select, test button |
