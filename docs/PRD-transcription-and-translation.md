# PRD: AI-Driven Transcription

## 1. Introduction/Overview

Pisum Langue is a cross-platform desktop utility (Windows and macOS) that lets users dictate text anywhere on their system. The user holds a global hotkey to record (push-to-talk) and releases it to stop. The app records the speech to a compressed audio file (Opus preferred), sends it with an optional prompt to a speech-to-text AI provider, copies the resulting text to the clipboard, and pastes it at the current cursor position.

## 2. Goals

- Provide a system-wide hotkey that triggers speech recording from the default microphone
- Record speech to a compressed audio format (Opus preferred, MP3 as fallback)
- Send the recorded audio with a prompt to a configurable speech-to-text AI provider
- Copy the transcription result to the system clipboard
- Automatically paste the result at the current cursor position
- Keep the AI provider abstracted so it can be replaced or upgraded independently
- Support both Windows and macOS

## 3. User Stories

1. As a user, I want to hold a hotkey and speak so that I can dictate text without switching applications.
2. As a user, I want the transcribed text to appear at my cursor position so that I can dictate directly into any text field or editor.
3. As a user, I want the transcribed text copied to my clipboard so that I can paste it manually if automatic pasting fails.
4. As a user, I want to configure which AI provider and model to use so that I can choose the best option for accuracy, speed, or cost.

## 4. Functional Requirements

### Recording

1. The system must register a configurable global hotkey that works across all applications on Windows and macOS.
2. The system must capture audio from the default system microphone when the hotkey is activated.
3. The system must encode the captured audio to a compressed format (Opus preferred, MP3 as fallback).
4. The system must stop recording when the hotkey is released (push-to-talk mode).
5. The system must provide audio/visual feedback (e.g., system tray icon change, small overlay) to indicate recording state.

### Transcription

1. The system must send the recorded audio file to a speech-to-text AI provider along with a configurable prompt.
2. The system must support configurable prompts that can guide the transcription (e.g., vocabulary hints, formatting instructions).
3. The AI provider must be abstracted behind an interface (`ITranscriptionService`) so the implementation can be swapped without modifying consuming code.

### Output

1. The system must copy the transcription result to the system clipboard.
2. The system must simulate a paste action (Ctrl+V / Cmd+V) to insert the text at the current cursor position.

### Error Handling & Offline Behavior

1. The system must detect when the network is unavailable or the transcription API call fails.
2. The system must show an OS-native toast notification (Windows toast notification / macOS NSUserNotification) with a clear error message when transcription fails for any reason (network, auth, quota, etc.).
3. The system must not silently discard errors — every failure in the pipeline (recording, encoding, transcription, pasting) must surface a notification to the user.

### Configuration

1. The system must provide a settings UI (accessible from system tray) to configure: hotkey, audio format, AI provider credentials, and prompt.
2. The system must persist settings between sessions.
3. The system must start minimized to the system tray / menu bar.

## 5. Non-Goals (Out of Scope)

- Not included: A full windowed UI for reviewing, editing, or exporting transcriptions
- Not included: File upload — the only input is live microphone recording
- Not included: SRT, TXT, or DOCX export
- Not included: Translation — the app is transcription-only
- Not included: Clipboard restoration — the transcription result overwrites the current clipboard content
- Not included: Streaming/real-time transcription during recording (audio is sent after recording stops)
- Not included: User accounts, authentication, or multi-user support
- Not included: Mobile platform support

## 6. Design Considerations

- The app runs as a system tray (Windows) / menu bar (macOS) application with no main window
- A small floating indicator or tray icon color change shows when recording is active
- Settings are accessed via right-click on the tray/menu bar icon
- The interaction should feel instantaneous — minimal latency between stopping recording and text appearing at the cursor
- Errors (network failure, invalid API key, no microphone, etc.) are surfaced as OS-native toast notifications (Windows toast / macOS NSUserNotification) so the user always gets feedback even when no app window is visible

## 7. Technical Considerations

- **Cross-platform:** Use .NET 10 (LTS) with Avalonia UI, plus platform-specific implementations for hotkey registration and clipboard/paste simulation.
- **Global Hotkey:** Requires OS-level hotkey registration (Win32 API on Windows, CGEvent on macOS).
- **Audio Recording:** Use platform audio APIs (e.g., NAudio on Windows, AVFoundation on macOS) to capture from the default microphone.
- **Audio Encoding:** Encode to Opus (preferred for size/quality) or MP3 as fallback, using a library like Concentus (Opus).
- **Clipboard & Paste Simulation:** Use OS clipboard APIs to set text, then simulate Ctrl+V / Cmd+V via input simulation (SendInput on Windows, CGEvent on macOS). The transcription overwrites the current clipboard content (no restore).
- **Notifications:** Use OS-native toast/notification APIs to surface errors (network, auth, device). On Windows use `ToastNotificationManager`; on macOS use `NSUserNotificationCenter` or `UNUserNotificationCenter`.
- **AI Provider Abstraction:** Define `ITranscriptionService` interfaces. Register implementations via the .NET DI container.
- **Dependency Injection:** Use the built-in .NET DI container to register services, making provider swaps a configuration change.

## 8. Success Metrics

- End-to-end latency (hotkey release → text pasted) under 3 seconds for short utterances (< 15 seconds of speech)
- Transcription works reliably across common applications (browsers, editors, chat apps, Office)
- Swapping the AI provider requires only adding a new implementation class and changing DI registration
- The app runs unobtrusively in the system tray with minimal resource usage when idle

## 9. Open Questions

- [x] Which cross-platform .NET framework should be used — .NET MAUI, Avalonia, or platform-specific builds? -> Avalonia
- [x] Should the hotkey default to push-to-talk (hold to record) or toggle (press to start/stop)? -> push-to-talk only
- [x] What is the maximum recording duration to support? -> 10 min
- [x] Which AI provider should be the default implementation (OpenAI Whisper, Azure, Google)? -> Google
- [x] Should the app support multiple prompt presets (e.g., "medical terminology", "casual conversation")? -> No
- [x] How should clipboard restoration work — always restore, or make it configurable? -> No clipboard restoration; transcription overwrites clipboard
