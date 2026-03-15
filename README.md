# Pisum Transcript

Hotkey-driven, system-tray-style dictation tool. Hold a global hotkey to record speech, release to transcribe via AI, and the result is pasted at the cursor position.

## Features

- **Push-to-talk recording** — Hold a configurable global hotkey (default: Ctrl+Shift+Space / Cmd+Shift+Space) to record, release to transcribe
- **AI transcription** — Audio sent to Google Gemini (extensible provider system) with customizable system prompts
- **Preset system** — Built-in and custom presets for different transcription modes (e.g. transcribe, translate)
- **Audio encoding** — Opus (preferred) with WAV fallback; configurable in settings
- **System tray** — Idle/recording icon states, active preset tooltip, notification control
- **Auto-start** — Optional launch at system startup
- **Hotkey conflict detection** — Warns about conflicts with system shortcuts

## Tech Stack

- **Backend**: Tauri 2 (Rust)
- **Frontend**: Svelte 5 (TypeScript), Vite 6, Tailwind CSS
- **AI Provider**: Google Gemini (configurable)

## Prerequisites

- [Node.js](https://nodejs.org/) 24+ (see `.nvmrc`)
- [Rust](https://rustup.rs/) 1.70+
- Platform-specific:
  - **Windows**: Visual Studio Build Tools
  - **macOS**: Xcode Command Line Tools, Opus (`brew install opus`)

## Development

```bash
npm install            # Install dependencies
npm run tauri:dev      # Start app in dev mode
npm run tauri:build    # Build application
npm run check          # Type-check Svelte/TypeScript
```

## Configuration

Settings are stored as JSON in the platform config directory and managed via the settings UI (accessible from the system tray).

| Setting              | Default          | Description                              |
| -------------------- | ---------------- | ---------------------------------------- |
| Hotkey               | Ctrl+Shift+Space | Global push-to-talk key                  |
| Audio format         | Opus             | Opus or WAV (with automatic fallback)    |
| Start with system    | On               | Launch at login                          |
| Tray notifications   | On               | Show OS notifications for status updates |

## macOS Permissions

- **Accessibility**: Required for paste simulation. Grant in System Settings > Privacy & Security > Accessibility.
- **Microphone**: Required for audio recording. macOS will prompt on first use.
