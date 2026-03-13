# PRD: Start and Stop Recording Mode

## 1. Introduction/Overview

Currently, Pisum Langue uses a **hold-to-record** model: the user holds down a hotkey to record speech and releases it to stop recording, which triggers transcription and paste. This works well for short dictations but can be fatiguing for longer recordings since the user must keep the key held down.

This feature introduces a **toggle recording mode** as an alternative. In toggle mode, the user presses the hotkey once to start recording and presses it again to stop. Both modes (hold-to-record and toggle) will be available, and the user can choose their preferred mode in settings.

## 2. Goals

- Allow users to record for extended periods without physical strain from holding a key
- Provide a choice between hold-to-record and toggle modes to accommodate different workflows
- Maintain the existing transcription and paste behavior regardless of which recording mode is used

## 3. User Stories

1. **As a user who dictates long passages**, I want to press a hotkey once to start recording and again to stop, so that I don't have to hold the key down the entire time.

2. **As a user who prefers the current behavior**, I want to keep using hold-to-record mode, so that my existing workflow is not disrupted.

3. **As a user switching between modes**, I want to change the recording mode in settings, so that I can pick the mode that fits my current task.

4. **As a user in toggle mode**, I want the tray icon to visually indicate that recording is active, so that I have a persistent visual reminder.

## 4. Functional Requirements

### Recording Mode Setting

1. The system must provide a "Recording Mode" setting with two options: **Hold to Record** and **Toggle (Start/Stop)**.
2. The system must default to **Hold to Record** to preserve existing behavior.
3. The setting must be persisted across application restarts.
4. Changing the recording mode must take effect immediately without requiring an app restart.

### Toggle Mode Behavior

5. In toggle mode, pressing the hotkey once must **start** audio recording.
6. In toggle mode, pressing the hotkey a second time must **stop** audio recording and trigger transcription.
7. The system must use the **same hotkey** configured for hold-to-record; behavior changes based on the selected mode.
8. The system must ignore the hotkey release event in toggle mode (release should not stop recording).
9. The existing maximum recording duration limit (10 minutes) must still apply in toggle mode — recording auto-stops and transcribes when the limit is reached.
10. The existing minimum recording duration check (50ms) must still apply in toggle mode.
11. The system must prevent starting a new recording while transcription is in progress (same guard as hold mode).

### Visual Feedback

12. The tray icon must change to the recording state icon when recording starts (same as current behavior, applies to both modes).
13. The tray icon must revert to idle when recording stops (same as current behavior, applies to both modes).

### Hold-to-Record Mode (Existing — No Changes)

1. In hold-to-record mode, pressing the hotkey must start recording (existing behavior, unchanged).
2. In hold-to-record mode, releasing the hotkey must stop recording and trigger transcription (existing behavior, unchanged).

## 5. Non-Goals (Out of Scope)

- **Not included:** Separate hotkey bindings for each recording mode — the same hotkey is shared, behavior depends on the selected mode.
- **Not included:** A preview or confirmation dialog before pasting — transcription and paste happen automatically in both modes.
- **Not included:** Audible feedback (chimes/sounds) for recording state changes.
- **Not included:** A third "hybrid" mode where short-press toggles and long-press holds — only the two distinct modes are offered.

## 6. Design Considerations

### Settings UI

- Add a "Recording Mode" option to the settings UI, likely in the **General** or **Audio** tab.
- Use a radio group or segmented control with two options: "Hold to Record" and "Toggle (Start/Stop)".
- Include a brief description below each option:
  - Hold to Record: "Hold the hotkey to record. Release to transcribe and paste."
  - Toggle: "Press the hotkey to start recording. Press again to transcribe and paste."

### Hotkey Config Label

- The [HotkeyConfig.svelte](src/components/HotkeyConfig.svelte) description text currently reads "Hold this key combination to record, release to transcribe and paste." This must update dynamically based on the selected recording mode.

## 7. Technical Considerations

- **Hotkey event handling:** The current implementation in [manager.rs](src-tauri/src/hotkey/manager.rs) uses `HotKeyState::Pressed` and `HotKeyState::Released` events. Toggle mode must track internal state (idle → recording → idle) and only act on `Pressed` events, ignoring `Released`.
- **State management:** A new recording mode field is needed in the app configuration ([config/](src-tauri/src/config/)). The hotkey manager must read this setting to determine which behavior to use.
- **Tray integration:** The tray icon and tooltip updates in [tray.rs](src-tauri/src/tray.rs) already handle recording state transitions — these should work without changes for toggle mode.
- **Max duration timer:** The existing 10-minute auto-stop timer spawned on recording start should work identically in toggle mode.

## 8. Success Metrics

- **Functional correctness:** Both recording modes work reliably — hold-to-record behaves identically to current implementation, toggle mode correctly starts/stops on consecutive presses.
- **No regressions:** Existing hold-to-record users experience no change in behavior when the default mode is active.
- **Setting persistence:** Recording mode selection survives app restarts.

## 9. Open Questions

- [x] Should the max recording duration be configurable, or remain fixed at 10 minutes for both modes? -> yes
