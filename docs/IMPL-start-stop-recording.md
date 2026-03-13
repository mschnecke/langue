# Implementation Plan: Start and Stop Recording Mode

> Generated from: `docs/PRD-start-stop-recording.md`
> Date: 2026-03-13

## 1. Overview

This feature adds a **toggle recording mode** alongside the existing hold-to-record (push-to-talk) mode. In toggle mode, the user presses the hotkey once to start recording and presses again to stop and transcribe. The user selects their preferred mode in settings. The same hotkey is used for both modes — only the press/release behavior changes.

The feature also makes the maximum recording duration configurable (resolved from the PRD's open question).

The change touches three layers:
1. **Config schema** — new `RecordingMode` enum and `maxRecordingDuration` field
2. **Hotkey manager** — branching press/release logic based on the active mode
3. **Frontend settings UI** — recording mode selector and max duration input

No new IPC commands are needed. The existing `save_settings` / `load_settings` flow carries the new fields automatically through `AppSettings`.

## 2. Architecture & Design

### State Machine

**Hold-to-record (existing):**
```
Idle ──[press]──► Recording ──[release]──► Transcribing ──► Idle
```

**Toggle mode (new):**
```
Idle ──[press]──► Recording ──[press]──► Transcribing ──► Idle
                                          (release events ignored)
```

Both modes share the same guards (no recording while transcribing, min duration check, max duration auto-stop) and the same transcription pipeline (`process_and_transcribe`).

### Data Flow for Mode Selection

```
User selects mode in GeneralConfig.svelte
  → onUpdate({ ...settings, recordingMode: 'toggle' })
  → persistSettings() → IPC save_settings
  → apply_settings() → SETTINGS RwLock updated
  → hotkey manager reads SETTINGS.recording_mode on next press/release event
```

No hotkey re-registration is needed when the mode changes — only the event handling logic branches.

## 3. Phases & Milestones

### Phase 1: Backend — Config & Hotkey Logic
**Goal:** Recording mode and configurable max duration work end-to-end via the Rust backend
**Deliverable:** Toggle mode functional when `recording_mode` is set to `toggle` in the JSON config file manually

### Phase 2: Frontend — Settings UI
**Goal:** User can select recording mode and configure max duration from the settings window
**Deliverable:** Full feature usable from the UI with dynamic labels

## 4. Files Overview

### Files to Create

_No new files required._

### Files to Modify

| File Path | What Changes |
|-----------|-------------|
| `src-tauri/src/config/schema.rs` | Add `RecordingMode` enum, `recording_mode` and `max_recording_duration_secs` fields to `AppSettings` |
| `src-tauri/src/hotkey/manager.rs` | Branch press/release handlers based on recording mode; use configurable max duration |
| `src/lib/types.ts` | Add `recordingMode` and `maxRecordingDurationSecs` to `AppSettings` interface |
| `src/components/GeneralConfig.svelte` | Add recording mode selector and max duration input |
| `src/components/HotkeyConfig.svelte` | Dynamic description text based on recording mode |

## 5. Task Breakdown

### Phase 1: Backend — Config & Hotkey Logic

#### Task 1.1: Add RecordingMode to config schema

- **Files to modify:**
  - `src-tauri/src/config/schema.rs` — Add enum and fields
- **Implementation details:**
  - Add `RecordingMode` enum:
    ```rust
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub enum RecordingMode {
        HoldToRecord,
        Toggle,
    }
    ```
  - Add fields to `AppSettings`:
    ```rust
    #[serde(default = "default_recording_mode")]
    pub recording_mode: RecordingMode,

    #[serde(default = "default_max_recording_duration_secs")]
    pub max_recording_duration_secs: u64,
    ```
  - Add default functions:
    ```rust
    fn default_recording_mode() -> RecordingMode {
        RecordingMode::HoldToRecord
    }

    fn default_max_recording_duration_secs() -> u64 {
        600 // 10 minutes
    }
    ```
  - Update `Default for AppSettings` impl to include both new fields
- **Dependencies:** None
- **Acceptance criteria:** Existing config files without the new fields deserialize correctly using defaults. New fields round-trip through save/load.

#### Task 1.2: Update hotkey manager for toggle mode

- **Files to modify:**
  - `src-tauri/src/hotkey/manager.rs` — Modify press/release handlers, use configurable duration
- **Implementation details:**
  - Remove the `MAX_RECORDING_DURATION` constant. Instead, read from `SETTINGS`:
    ```rust
    let max_secs = crate::SETTINGS
        .read()
        .map(|s| s.max_recording_duration_secs)
        .unwrap_or(600);
    let max_duration = Duration::from_secs(max_secs);
    ```
  - Read recording mode from `SETTINGS` in the event loop dispatch:
    ```rust
    let mode = crate::SETTINGS
        .read()
        .map(|s| s.recording_mode.clone())
        .unwrap_or(crate::config::schema::RecordingMode::HoldToRecord);
    ```
  - **For `HotKeyState::Pressed`:** Branch on mode:
    - `HoldToRecord` → call `handle_hotkey_press()` (existing behavior)
    - `Toggle` → check if currently recording:
      - Not recording → call `handle_hotkey_press()` (start recording)
      - Already recording → call `stop_and_transcribe()` (stop and process)
  - **For `HotKeyState::Released`:** Branch on mode:
    - `HoldToRecord` → call `handle_hotkey_release()` (existing behavior)
    - `Toggle` → do nothing (ignore release)
  - Extract the stop-and-transcribe logic from `handle_hotkey_release()` into a new function `stop_and_transcribe()` that both `handle_hotkey_release()` and the toggle-press-while-recording path call. This avoids duplicating the pipeline logic.
  - Update `handle_hotkey_press()` to pass `max_duration` to the timer thread instead of using the removed constant.
  - In toggle mode, the "already recording" guard in `handle_hotkey_press()` must be bypassed — this is handled by checking mode *before* calling `handle_hotkey_press()` in the event loop.
- **Dependencies:** Task 1.1
- **Acceptance criteria:**
  - With `recordingMode: "holdToRecord"` in config: behavior identical to current (press starts, release stops)
  - With `recordingMode: "toggle"` in config: first press starts, second press stops and transcribes, release events are ignored
  - Max duration auto-stop uses the configurable value
  - Transcription-in-progress guard works in both modes
  - Min duration check works in both modes

### Phase 2: Frontend — Settings UI

#### Task 2.1: Update TypeScript types

- **Files to modify:**
  - `src/lib/types.ts` — Add new fields
- **Implementation details:**
  ```typescript
  export interface AppSettings {
    // ... existing fields ...
    recordingMode: 'holdToRecord' | 'toggle';
    maxRecordingDurationSecs: number;
  }
  ```
- **Dependencies:** Task 1.1
- **Acceptance criteria:** Types match the Rust schema's JSON serialization

#### Task 2.2: Add recording mode and max duration to GeneralConfig

- **Files to modify:**
  - `src/components/GeneralConfig.svelte` — Add mode selector and duration input
- **Implementation details:**
  - Add a "Recording Mode" section after the existing toggles, using two radio-style buttons (matching the segmented control pattern used in `AudioConfig.svelte` for Opus/WAV):
    - **Hold to Record** — description: "Hold the hotkey to record. Release to transcribe and paste."
    - **Toggle (Start/Stop)** — description: "Press the hotkey to start recording. Press again to transcribe and paste."
  - Add a "Max Recording Duration" number input below the mode selector:
    - Label: "Maximum recording duration (seconds)"
    - Description: "Recording auto-stops after this duration."
    - Min: 10, Max: 3600 (1 hour), default: 600
    - On change: `onUpdate({ ...settings, maxRecordingDurationSecs: newValue })`
  - Mode change handler: `onUpdate({ ...settings, recordingMode: newMode })`
- **Dependencies:** Task 2.1
- **Acceptance criteria:**
  - Mode selector displays with correct initial value from settings
  - Changing mode persists immediately (no restart needed)
  - Duration input validates min/max bounds
  - Both settings survive app restart

#### Task 2.3: Dynamic hotkey description text

- **Files to modify:**
  - `src/components/HotkeyConfig.svelte` — Dynamic description based on mode
- **Implementation details:**
  - The static text on line 37 (`"Hold this key combination to record, release to transcribe and paste."`) becomes dynamic:
    ```svelte
    <p class="text-xs text-gray-500">
      {#if settings.recordingMode === 'toggle'}
        Press this key combination to start recording. Press again to transcribe and paste.
      {:else}
        Hold this key combination to record, release to transcribe and paste.
      {/if}
    </p>
    ```
- **Dependencies:** Task 2.1
- **Acceptance criteria:** Description text updates when recording mode changes in settings

## 6. Data Model Changes

No database changes. Two new fields added to the `AppSettings` JSON config file (`~/.pisum-langue.json`):

```json
{
  "recordingMode": "holdToRecord",
  "maxRecordingDurationSecs": 600
}
```

Both fields use `#[serde(default)]` so existing config files without them will deserialize with defaults (`holdToRecord`, `600`). No migration needed.

## 7. API Changes

No new IPC commands. The existing `save_settings` and `load_settings` commands automatically carry the new fields through the `AppSettings` struct.

## 8. Dependencies & Risks

- **No new crate dependencies.** The toggle logic is pure state management using existing `AtomicBool` and `Mutex` primitives.
- **Risk: Key repeat events.** On some OS/keyboard configurations, holding a key fires repeated `Pressed` events. In toggle mode, this could rapidly toggle recording on/off. **Mitigation:** Add a debounce guard — ignore `Pressed` events within 200ms of the last `Pressed` event in toggle mode.
- **Risk: Max duration timer race condition.** The existing timer thread calls `handle_hotkey_release()`. After refactoring, it should call `stop_and_transcribe()` directly to work correctly in both modes. This is addressed in Task 1.2.

## 9. Testing Strategy

### Manual Test Scenarios

1. **Hold-to-record default:** Fresh install → hold hotkey → release → verify transcription and paste (regression)
2. **Toggle mode basic:** Switch to toggle → press hotkey → tray icon changes → press again → transcription + paste
3. **Toggle ignores release:** In toggle mode, press and release quickly → recording should continue (not stop on release)
4. **Min duration in toggle:** In toggle mode, press → press again immediately (<50ms) → recording discarded
5. **Max duration auto-stop:** Set max duration to 10s → start toggle recording → wait 10s → auto-stops and transcribes
6. **Transcription guard:** Start toggle recording → stop (transcription begins) → press again immediately → should show "transcription in progress" notification
7. **Mode switch persistence:** Set to toggle → close app → reopen → verify mode is still toggle
8. **Key repeat debounce:** In toggle mode, hold the hotkey down → should not rapidly toggle (start once, ignore repeats)
9. **Configurable duration:** Change max duration to 30s → verify both modes respect the new limit

### Edge Cases

- Switch mode while recording is active (should not be possible if settings window isn't reachable during recording, but verify gracefully)
- Config file missing new fields (defaults applied correctly)
- Max duration set to minimum (10s) — recording auto-stops correctly

## 10. Requirement Traceability

### Functional Requirements

| PRD Ref | Requirement Summary | Task(s) | Notes |
|---------|-------------------|---------|-------|
| 4 #1 | Recording Mode setting with two options | 1.1, 2.2 | Enum in config + UI selector |
| 4 #2 | Default to Hold to Record | 1.1 | `default_recording_mode()` returns `HoldToRecord` |
| 4 #3 | Persisted across restarts | 1.1 | Serde JSON persistence via existing `save_settings` |
| 4 #4 | Immediate effect, no restart | 1.2 | Mode read from `SETTINGS` on each event |
| 4 #5 | Toggle press starts recording | 1.2 | Toggle branch in event loop |
| 4 #6 | Toggle second press stops and transcribes | 1.2 | `stop_and_transcribe()` called on press-while-recording |
| 4 #7 | Same hotkey for both modes | 1.2 | Single hotkey, mode-based branching |
| 4 #8 | Ignore release in toggle mode | 1.2 | Toggle branch returns early on `Released` |
| 4 #9 | Max duration applies in toggle mode | 1.2 | Timer uses configurable `max_recording_duration_secs` |
| 4 #10 | Min duration check in toggle mode | 1.2 | `stop_and_transcribe()` includes min duration guard |
| 4 #11 | Prevent recording during transcription | 1.2 | `IS_TRANSCRIBING` guard unchanged |
| 4 #12 | Tray icon changes on start | — | Existing `tray::set_recording_state(true)` — no changes needed |
| 4 #13 | Tray icon reverts on stop | — | Existing `tray::set_recording_state(false)` — no changes needed |
| 4 Hold #1 | Hold press starts recording | 1.2 | Existing behavior preserved in `HoldToRecord` branch |
| 4 Hold #2 | Hold release stops and transcribes | 1.2 | Existing behavior preserved in `HoldToRecord` branch |
| OQ (resolved) | Max duration configurable | 1.1, 1.2, 2.2 | New `maxRecordingDurationSecs` field + UI input |

### User Stories

| PRD Ref | User Story Summary | Implementing Tasks | Fully Covered? |
|---------|-------------------|-------------------|----------------|
| US-1 | Press once to start, again to stop | 1.2 | Yes |
| US-2 | Keep using hold-to-record | 1.1, 1.2 | Yes — default mode unchanged |
| US-3 | Change mode in settings | 2.2 | Yes |
| US-4 | Tray icon shows recording state | — | Yes — existing behavior, no changes needed |

### Success Metrics

| Metric | How the Plan Addresses It |
|--------|--------------------------|
| Functional correctness | Task 1.2 implements both modes with shared pipeline; testing strategy covers both |
| No regressions | Hold-to-record is default; existing logic preserved in its branch |
| Setting persistence | Serde default functions ensure backward compatibility; Task 2.2 tests persistence |
