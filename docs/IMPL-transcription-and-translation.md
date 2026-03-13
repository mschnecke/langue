# Implementation Plan: AI-Driven Transcription

> Generated from: `docs/PRD-transcription-and-translation.md`
> Date: 2026-03-13

## 1. Overview

Pisum Langue is a cross-platform desktop utility (Windows and macOS) that runs as a system tray / menu bar application. Users hold a global hotkey to record audio from the default microphone (push-to-talk). On release, the recorded audio is compressed (Opus preferred), sent to a configurable AI speech-to-text provider (Google by default) with an optional prompt, and the transcription result is copied to the clipboard and pasted at the current cursor position. Errors (network, auth, device) are surfaced via OS-native toast notifications.

The project is greenfield — no code exists yet. The tech stack is **.NET 10 (LTS) with Avalonia UI** (per PRD open-question resolution). Architecture follows clean separation with dependency injection, abstracting the AI provider behind an `ITranscriptionService` interface. Both Windows and macOS platform implementations are developed in parallel from the start.

## 2. Architecture & Design

### High-Level Component Diagram

```text
┌─────────────────────────────────────────────────────────┐
│                    Avalonia UI Layer                      │
│  ┌──────────────┐  ┌──────────────┐  ┌───────────────┐  │
│  │  System Tray  │  │  Settings UI │  │ Recording     │  │
│  │  Icon/Menu    │  │  Window      │  │ Indicator     │  │
│  └──────┬───────┘  └──────┬───────┘  └───────┬───────┘  │
└─────────┼─────────────────┼───────────────────┼──────────┘
          │                 │                   │
┌─────────┼─────────────────┼───────────────────┼──────────┐
│         ▼                 ▼                   ▼          │
│  ┌─────────────────────────────────────────────────────┐ │
│  │              Application Services                    │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌───────────┐  │ │
│  │  │ HotkeyService│  │SettingsService│  │Orchestrator│  │ │
│  │  └──────────────┘  └──────────────┘  └───────────┘  │ │
│  └─────────────────────────────────────────────────────┘ │
│                                                          │
│  ┌─────────────────────────────────────────────────────┐ │
│  │              Core Services                           │ │
│  │  ┌────────────┐ ┌──────────────┐ ┌───────────────┐  │ │
│  │  │AudioRecorder│ │Transcription │ │ Clipboard &   │  │ │
│  │  │  Service    │ │  Service     │ │ Paste Service │  │ │
│  │  └────────────┘ └──────────────┘ └───────────────┘  │ │
│  └─────────────────────────────────────────────────────┘ │
│                    Core / Domain Layer                    │
└──────────────────────────────────────────────────────────┘
```

### Data Flow

```text
Hotkey Down (Push-to-Talk)
  → AudioRecorderService.StartRecording()
  → Capture from default mic → buffer in memory

Hotkey Up (Release)
  → AudioRecorderService.StopRecording()
  → Encode to Opus (or MP3 fallback)
  → Save temp file

  → ITranscriptionService.TranscribeAsync(audioFile, prompt)

  → ClipboardService.SetText(result)
  → PasteSimulatorService.Paste()     // Ctrl+V / Cmd+V

  On error at any step:
  → INotificationService.ShowError(message)  // OS-native toast notification
```

### Key Abstractions

```csharp
public interface ITranscriptionService
{
    Task<TranscriptionResult> TranscribeAsync(
        string audioFilePath,
        string? prompt = null,
        CancellationToken ct = default);
}

public interface IAudioRecorderService
{
    void StartRecording();
    Task<string> StopRecordingAsync(); // returns path to compressed audio file
    bool IsRecording { get; }
    event EventHandler<RecordingStateChangedEventArgs>? StateChanged;
}

public interface IClipboardService
{
    Task SetTextAsync(string text);
}

public interface IPasteSimulatorService
{
    Task PasteAsync();
}

public interface INotificationService
{
    Task ShowNotificationAsync(string title, string message);
    Task ShowErrorAsync(string title, string message);
}

public interface IGlobalHotkeyService
{
    void Register(KeyCombination hotkey, Action onKeyDown, Action onKeyUp);
    void Unregister();
}

public interface ISettingsService
{
    AppSettings Current { get; }
    Task SaveAsync(AppSettings settings);
    event EventHandler<AppSettings>? SettingsChanged;
}
```

### Project Structure

```text
(repo root)/
├── Pisum.Langue.slnx
├── src/
│   ├── Pisum.Langue.Core/                  # Interfaces, models, shared logic
│   │   ├── Pisum.Langue.Core.csproj
│   │   ├── Interfaces/
│   │   ├── Models/
│   │   └── Extensions/
│   ├── Pisum.Langue.App/                   # Avalonia app, UI, DI composition root
│   │   ├── Pisum.Langue.App.csproj
│   │   ├── Program.cs
│   │   ├── App.axaml / App.axaml.cs
│   │   ├── Views/
│   │   ├── ViewModels/
│   │   ├── Services/
│   │   └── Assets/
│   ├── Pisum.Langue.Platform.Windows/      # Windows-specific implementations
│   │   ├── Pisum.Langue.Platform.Windows.csproj
│   │   └── Services/
│   └── Pisum.Langue.Platform.MacOS/        # macOS-specific implementations
│       ├── Pisum.Langue.Platform.MacOS.csproj
│       └── Services/
└── tests/
    ├── Pisum.Langue.Core.Tests/
    └── Pisum.Langue.App.Tests/
```

## 3. Phases & Milestones

### Phase 1: Project Scaffold & Core Abstractions

**Goal:** Establish the solution structure, core interfaces, models, and platform project stubs for both Windows and macOS. The app builds and runs showing a system tray icon on both platforms.

**Deliverable:** Solution compiles on both platforms, tray icon visible, no functionality yet.

### Phase 2: Global Hotkey & Audio Recording (Windows + macOS)

**Goal:** Register a global hotkey with push-to-talk (hold to record, release to stop) and record audio from the default microphone to an Opus-compressed file on both platforms.

**Deliverable:** User holds hotkey → recording starts → releases → Opus audio file saved to temp directory. Works on both Windows and macOS.

### Phase 3: Transcription Service Integration

**Goal:** Send recorded audio to Google Speech-to-Text and receive transcription text. The prompt can include formatting/vocabulary instructions.

**Deliverable:** Audio file → API call → transcription text returned and logged.

### Phase 4: Clipboard & Paste Output (Windows + macOS)

**Goal:** Copy transcription to clipboard and simulate paste at cursor position on both platforms.

**Deliverable:** After transcription, text appears at the cursor position in any application on both Windows and macOS.

### Phase 5: Settings UI & Persistence

**Goal:** Provide a settings window accessible from the tray icon to configure hotkey, provider credentials, audio format, prompt, and preferences.

**Deliverable:** Fully functional settings window with persistent configuration.

### Phase 6: Recording Feedback & Polish

**Goal:** Visual/audio feedback during recording, error handling with OS-native toast notifications, and overall polish.

**Deliverable:** Production-ready feel with indicator overlay and error notifications on both platforms.

## 4. Files Overview

### Files to Create

| File Path                                                                    | Purpose                                             |
| ---------------------------------------------------------------------------- | --------------------------------------------------- |
| `Pisum.Langue.slnx`                                                            | Solution file                                       |
| `src/Pisum.Langue.Core/Pisum.Langue.Core.csproj`                               | Core library project file                           |
| `src/Pisum.Langue.Core/Interfaces/ITranscriptionService.cs`                   | Transcription provider abstraction                  |
| `src/Pisum.Langue.Core/Interfaces/IAudioRecorderService.cs`                   | Audio recording abstraction                         |
| `src/Pisum.Langue.Core/Interfaces/IClipboardService.cs`                       | Clipboard access abstraction                        |
| `src/Pisum.Langue.Core/Interfaces/IPasteSimulatorService.cs`                  | Paste simulation abstraction                        |
| `src/Pisum.Langue.Core/Interfaces/IGlobalHotkeyService.cs`                    | Global hotkey abstraction                           |
| `src/Pisum.Langue.Core/Interfaces/ISettingsService.cs`                        | Settings persistence abstraction                    |
| `src/Pisum.Langue.Core/Interfaces/IAudioEncoderService.cs`                    | Audio encoding abstraction                          |
| `src/Pisum.Langue.Core/Interfaces/INotificationService.cs`                   | OS-native toast notification abstraction            |
| `src/Pisum.Langue.Core/Models/AppSettings.cs`                                 | Settings model                                      |
| `src/Pisum.Langue.Core/Models/ProviderConfig.cs`                              | Provider configuration (name, type, API key)        |
| `src/Pisum.Langue.Core/Models/TranscriptionResult.cs`                         | Transcription response model                        |
| `src/Pisum.Langue.Core/Models/RecordingStateChangedEventArgs.cs`              | Recording state event args                          |
| `src/Pisum.Langue.Core/Models/KeyCombination.cs`                              | Hotkey combination model                            |
| `src/Pisum.Langue.Core/Models/AudioFormat.cs`                                 | Enum for Opus/MP3                                   |
| `src/Pisum.Langue.App/Pisum.Langue.App.csproj`                                 | Avalonia application project file                   |
| `src/Pisum.Langue.App/Program.cs`                                             | Application entry point                             |
| `src/Pisum.Langue.App/App.axaml`                                              | Avalonia app XAML                                   |
| `src/Pisum.Langue.App/App.axaml.cs`                                           | Avalonia app code-behind, DI composition root       |
| `src/Pisum.Langue.App/Views/SettingsWindow.axaml`                             | Settings window XAML                                |
| `src/Pisum.Langue.App/Views/SettingsWindow.axaml.cs`                          | Settings window code-behind                         |
| `src/Pisum.Langue.App/ViewModels/SettingsViewModel.cs`                        | Settings window view model                          |
| `src/Pisum.Langue.App/Services/TranscriptionOrchestrator.cs`                  | Coordinates record → transcribe → paste workflow    |
| `src/Pisum.Langue.App/Services/SettingsService.cs`                            | JSON-based settings persistence                     |
| `src/Pisum.Langue.App/Services/AudioEncoderService.cs`                        | Cross-platform audio encoding (Opus preferred)      |
| `src/Pisum.Langue.App/Services/GoogleTranscriptionService.cs`                 | Google Speech-to-Text implementation                |
| `src/Pisum.Langue.App/Services/RoundRobinTranscriptionService.cs`             | Wraps multiple providers, cycles through them       |
| `src/Pisum.Langue.Platform.Windows/Pisum.Langue.Platform.Windows.csproj`       | Windows platform project                            |
| `src/Pisum.Langue.Platform.Windows/Services/WindowsGlobalHotkeyService.cs`    | Win32 low-level keyboard hook for push-to-talk      |
| `src/Pisum.Langue.Platform.Windows/Services/WindowsAudioRecorderService.cs`   | NAudio-based recording                              |
| `src/Pisum.Langue.Platform.Windows/Services/WindowsClipboardService.cs`       | Windows clipboard access                            |
| `src/Pisum.Langue.Platform.Windows/Services/WindowsPasteSimulatorService.cs`  | SendInput-based paste                               |
| `src/Pisum.Langue.Platform.Windows/Services/WindowsNotificationService.cs`   | Windows toast notification (ToastNotificationManager) |
| `src/Pisum.Langue.Platform.MacOS/Pisum.Langue.Platform.MacOS.csproj`           | macOS platform project                              |
| `src/Pisum.Langue.Platform.MacOS/Services/MacOSGlobalHotkeyService.cs`        | CGEvent-based push-to-talk hotkey                   |
| `src/Pisum.Langue.Platform.MacOS/Services/MacOSAudioRecorderService.cs`       | AVFoundation-based recording                        |
| `src/Pisum.Langue.Platform.MacOS/Services/MacOSClipboardService.cs`           | macOS clipboard (NSPasteboard)                      |
| `src/Pisum.Langue.Platform.MacOS/Services/MacOSPasteSimulatorService.cs`      | CGEvent-based paste (Cmd+V)                         |
| `src/Pisum.Langue.Platform.MacOS/Services/MacOSNotificationService.cs`       | macOS notification (NSUserNotificationCenter)       |
| `tests/Pisum.Langue.Core.Tests/Pisum.Langue.Core.Tests.csproj`                 | Core unit test project                              |
| `tests/Pisum.Langue.App.Tests/Pisum.Langue.App.Tests.csproj`                   | App unit test project                               |
| `tests/Pisum.Langue.App.Tests/Services/TranscriptionOrchestratorTests.cs`     | Orchestrator unit tests                             |
| `tests/Pisum.Langue.App.Tests/Services/SettingsServiceTests.cs`               | Settings persistence tests                          |

### Files to Modify

| File Path   | What Changes                                                 |
| ----------- | ------------------------------------------------------------ |
| `CLAUDE.md` | Update build commands and architecture section as project evolves |

## 5. Task Breakdown

### Phase 1 Tasks: Project Scaffold & Core Abstractions

#### Task 1.1: Create Solution and Project Structure

- **Files to create/modify:**
  - `Pisum.Langue.slnx` — solution file with all projects
  - `src/Pisum.Langue.Core/Pisum.Langue.Core.csproj` — `net10.0` class library
  - `src/Pisum.Langue.App/Pisum.Langue.App.csproj` — `net10.0` Avalonia app, references Core
  - `src/Pisum.Langue.Platform.Windows/Pisum.Langue.Platform.Windows.csproj` — `net10.0-windows`, references Core
  - `src/Pisum.Langue.Platform.MacOS/Pisum.Langue.Platform.MacOS.csproj` — `net10.0-macos`, references Core
  - `tests/Pisum.Langue.Core.Tests/Pisum.Langue.Core.Tests.csproj` — xUnit test project
  - `tests/Pisum.Langue.App.Tests/Pisum.Langue.App.Tests.csproj` — xUnit test project
- **Implementation details:**
  - Use `dotnet new sln`, `dotnet new classlib`, `dotnet new avalonia.app` templates
  - Target `net10.0` (LTS)
  - Add NuGet references: `Avalonia`, `Avalonia.Desktop`, `Avalonia.Themes.Fluent`
  - Configure `Directory.Build.props` for shared settings (nullable, implicit usings, TreatWarningsAsErrors)
- **Dependencies:** None
- **Acceptance criteria:** `dotnet build` succeeds for the entire solution

#### Task 1.2: Define Core Interfaces

- **Files to create:**
  - `src/Pisum.Langue.Core/Interfaces/ITranscriptionService.cs` — `TranscribeAsync(string audioFilePath, string? prompt, CancellationToken)` returning `TranscriptionResult`
  - `src/Pisum.Langue.Core/Interfaces/IAudioRecorderService.cs` — `StartRecording()`, `StopRecordingAsync()`, `IsRecording`, `StateChanged` event
  - `src/Pisum.Langue.Core/Interfaces/IClipboardService.cs` — `GetTextAsync()`, `SetTextAsync(string)`
  - `src/Pisum.Langue.Core/Interfaces/IPasteSimulatorService.cs` — `PasteAsync()`
  - `src/Pisum.Langue.Core/Interfaces/IGlobalHotkeyService.cs` — `Register(KeyCombination, Action onKeyDown, Action onKeyUp)`, `Unregister()`
  - `src/Pisum.Langue.Core/Interfaces/ISettingsService.cs` — `Current`, `SaveAsync()`, `SettingsChanged` event
  - `src/Pisum.Langue.Core/Interfaces/IAudioEncoderService.cs` — `EncodeAsync(Stream pcmAudio, AudioFormat format, string outputPath)`
  - `src/Pisum.Langue.Core/Interfaces/INotificationService.cs` — `ShowNotificationAsync(title, message)`, `ShowErrorAsync(title, message)` for OS-native toast notifications
- **Implementation details:** See interface signatures in Section 2 above
- **Dependencies:** Task 1.1
- **Acceptance criteria:** All interfaces compile, no implementations yet

#### Task 1.3: Define Core Models

- **Files to create:**
  - `src/Pisum.Langue.Core/Models/AppSettings.cs` — properties: `Hotkey`, `AudioFormat`, `Providers` (list of `ProviderConfig`), `Prompt`, `MaxRecordingDurationSeconds` (default 600)
  - `src/Pisum.Langue.Core/Models/ProviderConfig.cs` — properties: `Name`, `Type` (e.g. "Google"), `ApiKey`, `Enabled`
  - `src/Pisum.Langue.Core/Models/TranscriptionResult.cs` — `Text`, `Duration`, `Success`, `ErrorMessage`
  - `src/Pisum.Langue.Core/Models/RecordingStateChangedEventArgs.cs` — `IsRecording`, `Duration`
  - `src/Pisum.Langue.Core/Models/KeyCombination.cs` — `Key`, `Modifiers`
  - `src/Pisum.Langue.Core/Models/AudioFormat.cs` — enum: `Opus`, `Mp3`
- **Dependencies:** Task 1.1
- **Acceptance criteria:** Models compile, `AppSettings` has sensible defaults

#### Task 1.4: Avalonia App Bootstrap with System Tray

- **Files to create/modify:**
  - `src/Pisum.Langue.App/Program.cs` — entry point, configure Avalonia builder
  - `src/Pisum.Langue.App/App.axaml` — app resources, Fluent theme
  - `src/Pisum.Langue.App/App.axaml.cs` — `OnFrameworkInitializationCompleted`: set up DI container, create `TrayIcon`, set `MainWindow = null` (tray-only mode)
  - `src/Pisum.Langue.App/Assets/tray-icon.ico` — placeholder tray icon
- **Implementation details:**
  - Use `Avalonia.Controls.TrayIcon` for the system tray
  - Right-click menu items: "Settings", "Exit"
  - Start with no main window (tray-only)
  - Configure `IServiceCollection` / `IServiceProvider` in `App.axaml.cs`
  - Use `RuntimeInformation.IsOSPlatform()` to register platform-specific DI services
- **Dependencies:** Task 1.1
- **Acceptance criteria:** App launches minimized to tray, icon visible, right-click shows "Settings" and "Exit", "Exit" closes the app

### Phase 2 Tasks: Global Hotkey & Audio Recording (Windows + macOS)

#### Task 2.1: Windows Global Hotkey Service (Push-to-Talk)

- **Files to create:**
  - `src/Pisum.Langue.Platform.Windows/Services/WindowsGlobalHotkeyService.cs`
- **Implementation details:**
  - Use low-level keyboard hook (`SetWindowsHookEx` with `WH_KEYBOARD_LL`) to detect both key-down and key-up events for the configured hotkey. `RegisterHotKey` only fires on key-down and cannot detect release.
  - Invoke `onKeyDown` callback when hotkey is pressed, `onKeyUp` when released
  - Invoke callbacks on UI thread

  ```csharp
  [DllImport("user32.dll")]
  private static extern IntPtr SetWindowsHookEx(int idHook, LowLevelKeyboardProc lpfn, IntPtr hMod, uint dwThreadId);

  private delegate IntPtr LowLevelKeyboardProc(int nCode, IntPtr wParam, IntPtr lParam);
  private const int WH_KEYBOARD_LL = 13;
  private const int WM_KEYDOWN = 0x0100;
  private const int WM_KEYUP = 0x0101;
  ```

- **Dependencies:** Task 1.2, Task 1.4
- **Acceptance criteria:** Holding hotkey fires key-down; releasing fires key-up; works while any application is focused

#### Task 2.2: macOS Global Hotkey Service (Push-to-Talk)

- **Files to create:**
  - `src/Pisum.Langue.Platform.MacOS/Services/MacOSGlobalHotkeyService.cs`
- **Implementation details:**
  - Use `CGEvent` tap via P/Invoke or `NSEvent.AddGlobalMonitorForEvents` to detect key-down and key-up
  - Request Accessibility permissions (required for global event monitoring)
  - Invoke `onKeyDown` / `onKeyUp` callbacks on main thread
- **Dependencies:** Task 1.2, Task 1.4
- **Acceptance criteria:** Push-to-talk hotkey works on macOS; Accessibility permission prompt handled

#### Task 2.3: Windows Audio Recording Service

- **Files to create:**
  - `src/Pisum.Langue.Platform.Windows/Services/WindowsAudioRecorderService.cs`
- **Implementation details:**
  - Use **NAudio** (`WasapiCapture` or `WaveInEvent`) to capture from default microphone
  - Buffer PCM audio in memory (`MemoryStream` or temp WAV file)
  - On stop, pass buffer to encoder service
  - Enforce max recording duration (10 min) via timer
  - Raise `StateChanged` event on start/stop
  - NuGet: `NAudio` (latest stable)
- **Dependencies:** Task 1.2
- **Acceptance criteria:** Start/stop recording produces a valid PCM buffer; max duration enforced

#### Task 2.4: macOS Audio Recording Service

- **Files to create:**
  - `src/Pisum.Langue.Platform.MacOS/Services/MacOSAudioRecorderService.cs`
- **Implementation details:**
  - Use AVFoundation via .NET macOS bindings to capture from default microphone
  - Request Microphone permissions
  - Buffer PCM audio, enforce max recording duration (10 min)
  - Raise `StateChanged` event on start/stop
- **Dependencies:** Task 1.2
- **Acceptance criteria:** Records audio from default mic on macOS; max duration enforced

#### Task 2.5: Audio Encoder Service (Cross-Platform)

- **Files to create:**
  - `src/Pisum.Langue.App/Services/AudioEncoderService.cs`
- **Implementation details:**
  - Opus encoding via **Concentus** NuGet package (pure .NET, cross-platform)
  - MP3 as fallback via **NAudio.Lame** (Windows) or alternative on macOS
  - Prefer Opus as default — smallest file size with best quality for speech
  - Write compressed output to temp file, return path
  - NuGet: `Concentus`, `Concentus.OggFile`
- **Dependencies:** Task 2.3, Task 2.4
- **Acceptance criteria:** PCM audio buffer encodes to Opus file; file plays correctly in a media player

#### Task 2.6: Wire Hotkey to Push-to-Talk Recording

- **Files to create/modify:**
  - `src/Pisum.Langue.App/Services/TranscriptionOrchestrator.cs` — create this orchestrator
  - `src/Pisum.Langue.App/App.axaml.cs` — register DI services, wire orchestrator
- **Implementation details:**
  - `TranscriptionOrchestrator` registers with `IGlobalHotkeyService`
  - `onKeyDown` → `StartRecording()`, `onKeyUp` → `StopRecordingAsync()` → encode → (transcription comes in Phase 3)
  - Simple state: idle → recording → processing → idle
  - Ignore key-down if already recording; ignore key-up if not recording
- **Dependencies:** Task 2.1, Task 2.2, Task 2.5
- **Acceptance criteria:** Hold hotkey → recording; release → Opus file produced. Works on both platforms.

### Phase 3 Tasks: Transcription Service Integration

#### Task 3.1: Google Speech-to-Text Implementation

- **Files to create:**
  - `src/Pisum.Langue.App/Services/GoogleTranscriptionService.cs`
- **Implementation details:**
  - Implement `ITranscriptionService`
  - Use **Google.Cloud.Speech.V2** NuGet package (or REST API via `HttpClient`)
  - Send audio file with configurable prompt/context (vocabulary hints, formatting instructions)
  - Parse response into `TranscriptionResult`
  - Handle errors gracefully (network, auth, quota)
  - NuGet: `Google.Cloud.Speech.V2` or `Google.Apis.Speech.v1`

  ```csharp
  public class GoogleTranscriptionService : ITranscriptionService
  {
      private readonly HttpClient _httpClient;
      private readonly ProviderConfig _config;

      public GoogleTranscriptionService(HttpClient httpClient, ProviderConfig config)
      {
          _httpClient = httpClient;
          _config = config;
      }

      public async Task<TranscriptionResult> TranscribeAsync(
          string audioFilePath, string? prompt, CancellationToken ct)
      {
          // Read audio file, send to Google API using _config.ApiKey, parse response
      }
  }
  ```

- **Dependencies:** Task 1.2, Task 1.3
- **Acceptance criteria:** Given a valid audio file and API key, returns correct transcription text

#### Task 3.2: Round-Robin Transcription Service

- **Files to create:**
  - `src/Pisum.Langue.App/Services/RoundRobinTranscriptionService.cs`
- **Implementation details:**
  - Implements `ITranscriptionService` and wraps a list of provider instances
  - Builds the provider list from `AppSettings.Providers` (only enabled entries)
  - On each `TranscribeAsync` call, picks the next enabled provider in round-robin order (thread-safe index via `Interlocked.Increment`)
  - If the selected provider fails, falls back to the next provider in the list before surfacing an error
  - Rebuilds the provider list when `ISettingsService.SettingsChanged` fires

  ```csharp
  public class RoundRobinTranscriptionService : ITranscriptionService
  {
      private ITranscriptionService[] _providers;
      private int _index = -1;

      public async Task<TranscriptionResult> TranscribeAsync(
          string audioFilePath, string? prompt, CancellationToken ct)
      {
          var providers = _providers;
          if (providers.Length == 0)
              return TranscriptionResult.Failure("No providers configured");

          var idx = Interlocked.Increment(ref _index) % providers.Length;
          var result = await providers[idx].TranscribeAsync(audioFilePath, prompt, ct);

          if (!result.Success && providers.Length > 1)
          {
              // Try next provider as fallback
              idx = (idx + 1) % providers.Length;
              result = await providers[idx].TranscribeAsync(audioFilePath, prompt, ct);
          }

          return result;
      }
  }
  ```

- **Dependencies:** Task 3.1, Task 1.3
- **Acceptance criteria:** Calls cycle across enabled providers; if one fails, the next is tried

#### Task 3.3: Wire Transcription into Orchestrator

- **Files to modify:**
  - `src/Pisum.Langue.App/Services/TranscriptionOrchestrator.cs` — after recording stops, call `ITranscriptionService.TranscribeAsync()`
- **Implementation details:**
  - DI registers `RoundRobinTranscriptionService` as the `ITranscriptionService` implementation
  - After encoding completes, send audio file to transcription service (round-robin picks the provider)
  - Log result for now (clipboard/paste comes in Phase 4)
  - Show error notification via tray if transcription fails
  - Clean up temp audio file after transcription
- **Dependencies:** Task 2.6, Task 3.2
- **Acceptance criteria:** Full flow: hold hotkey → record → release → encode → transcribe (round-robin) → result logged

### Phase 4 Tasks: Clipboard & Paste Output (Windows + macOS)

#### Task 4.1: Windows Clipboard Service

- **Files to create:**
  - `src/Pisum.Langue.Platform.Windows/Services/WindowsClipboardService.cs`
- **Implementation details:**
  - Use Avalonia's `IClipboard` (preferred for cross-platform) or Win32 clipboard APIs
  - `SetTextAsync(string)` — set clipboard text with the transcription result
- **Dependencies:** Task 1.2
- **Acceptance criteria:** Can write text to clipboard

#### Task 4.2: Windows Paste Simulator Service

- **Files to create:**
  - `src/Pisum.Langue.Platform.Windows/Services/WindowsPasteSimulatorService.cs`
- **Implementation details:**
  - Use `SendInput` Win32 API to simulate Ctrl+V keypress

  ```csharp
  [DllImport("user32.dll")]
  private static extern uint SendInput(uint nInputs, INPUT[] pInputs, int cbSize);
  ```

  - Send: Ctrl down → V down → V up → Ctrl up
  - Add small delay after setting clipboard before pasting (allow clipboard to propagate)
- **Dependencies:** Task 1.2
- **Acceptance criteria:** Calling `PasteAsync()` after setting clipboard causes text to appear at cursor position in active application

#### Task 4.3: macOS Clipboard Service

- **Files to create:**
  - `src/Pisum.Langue.Platform.MacOS/Services/MacOSClipboardService.cs`
- **Implementation details:**
  - Use Avalonia's cross-platform `IClipboard` or `NSPasteboard` via P/Invoke
  - Same interface as Windows: `SetTextAsync(string)`
- **Dependencies:** Task 1.2
- **Acceptance criteria:** Can write text to clipboard on macOS

#### Task 4.4: macOS Paste Simulator Service

- **Files to create:**
  - `src/Pisum.Langue.Platform.MacOS/Services/MacOSPasteSimulatorService.cs`
- **Implementation details:**
  - Use `CGEvent` to simulate Cmd+V keypress
  - Send: Cmd down → V down → V up → Cmd up
- **Dependencies:** Task 1.2
- **Acceptance criteria:** Calling `PasteAsync()` after setting clipboard causes text to appear at cursor position on macOS

#### Task 4.5: Wire Clipboard & Paste into Orchestrator

- **Files to modify:**
  - `src/Pisum.Langue.App/Services/TranscriptionOrchestrator.cs`
- **Implementation details:**
  - After transcription: set transcription text on clipboard → simulate paste
  - On any error in the pipeline: show OS-native toast notification via `INotificationService`
  - Add configurable delay between clipboard set and paste

  ```csharp
  try
  {
      await _clipboard.SetTextAsync(transcriptionResult.Text);
      await Task.Delay(50); // allow clipboard to propagate
      await _pasteSimulator.PasteAsync();
  }
  catch (Exception ex)
  {
      await _notifications.ShowErrorAsync("Transcription Failed", ex.Message);
  }
  ```

- **Dependencies:** Task 3.3, Task 4.1, Task 4.2, Task 4.3, Task 4.4
- **Acceptance criteria:** Hold hotkey → record → transcribe → text appears at cursor. On failure, OS toast notification shown. Works on both platforms.

### Phase 5 Tasks: Settings UI & Persistence

#### Task 5.1: Settings Persistence Service

- **Files to create:**
  - `src/Pisum.Langue.App/Services/SettingsService.cs`
- **Implementation details:**
  - Store settings as JSON in `~/.pisum-langue/settings.json` (user home folder on both platforms)
  - Load on startup with sensible defaults
  - Save on change
  - Raise `SettingsChanged` event
  - Use `System.Text.Json` for serialization
- **Dependencies:** Task 1.2, Task 1.3
- **Acceptance criteria:** Settings persist between app restarts; missing file creates defaults

#### Task 5.2: Settings ViewModel

- **Files to create:**
  - `src/Pisum.Langue.App/ViewModels/SettingsViewModel.cs`
- **Implementation details:**
  - Bind to all `AppSettings` properties
  - Implement `INotifyPropertyChanged` (or use CommunityToolkit.Mvvm `ObservableObject`)
  - Save command, cancel command
  - Hotkey capture mode (press a key combination to set it)
  - NuGet: `CommunityToolkit.Mvvm`
- **Dependencies:** Task 5.1
- **Acceptance criteria:** ViewModel correctly reads/writes settings

#### Task 5.3: Settings Window UI

- **Files to create:**
  - `src/Pisum.Langue.App/Views/SettingsWindow.axaml` — XAML layout
  - `src/Pisum.Langue.App/Views/SettingsWindow.axaml.cs` — code-behind
- **Implementation details:**
  - Sections: Hotkey, Audio Format (Opus/MP3), Providers (list — add/remove/reorder, each with name, type, API key, enabled toggle), Prompt
  - Use Avalonia Fluent theme controls
  - Open from tray icon right-click "Settings"
  - Single-instance window (don't open multiple)
- **Dependencies:** Task 5.2, Task 1.4
- **Acceptance criteria:** Settings window opens from tray, all fields editable, changes persist

#### Task 5.4: DI Registration Updates for Settings

- **Files to modify:**
  - `src/Pisum.Langue.App/App.axaml.cs` — register `SettingsService`, use settings to configure other services
- **Implementation details:**
  - Load settings early in startup
  - Re-register hotkey when hotkey setting changes
  - Apply audio format setting to encoder
- **Dependencies:** Task 5.1, Task 2.6
- **Acceptance criteria:** Changing settings at runtime takes effect without restart (hotkey, format)

### Phase 6 Tasks: Recording Feedback & Polish

#### Task 6.1: Recording State Indicator

- **Files to create/modify:**
  - `src/Pisum.Langue.App/Assets/tray-icon-recording.ico` — red/active tray icon variant
  - `src/Pisum.Langue.App/App.axaml.cs` — swap tray icon based on recording state
- **Implementation details:**
  - Subscribe to `IAudioRecorderService.StateChanged`
  - Change tray icon when recording starts/stops
  - Optionally show a small floating overlay window (semi-transparent, topmost, click-through)
- **Dependencies:** Task 2.6
- **Acceptance criteria:** Tray icon visually changes when recording is active

#### Task 6.2: Windows Notification Service

- **Files to create:**
  - `src/Pisum.Langue.Platform.Windows/Services/WindowsNotificationService.cs`
- **Implementation details:**
  - Implement `INotificationService`
  - Use Windows `ToastNotificationManager` to show OS-native toast notifications
  - Show app name and icon in the notification
  - NuGet: `Microsoft.Toolkit.Uwp.Notifications` or use Win32 `Shell_NotifyIcon` balloon tips
- **Dependencies:** Task 1.2
- **Acceptance criteria:** Error and info messages appear as Windows toast notifications

#### Task 6.3: macOS Notification Service

- **Files to create:**
  - `src/Pisum.Langue.Platform.MacOS/Services/MacOSNotificationService.cs`
- **Implementation details:**
  - Implement `INotificationService`
  - Use `NSUserNotificationCenter` or `UNUserNotificationCenter` via P/Invoke
  - Show app name and icon in the notification
- **Dependencies:** Task 1.2
- **Acceptance criteria:** Error and info messages appear as macOS notifications

#### Task 6.4: Error Handling & Notification Wiring

- **Files to modify:**
  - `src/Pisum.Langue.App/Services/TranscriptionOrchestrator.cs` — wrap pipeline in try/catch, show notifications
- **Implementation details:**
  - Use `INotificationService` to show OS-native toast notifications on errors (network failure, invalid API key, audio device unavailable)
  - Log errors to file (`~/.pisum-langue/logs/`)
  - Every failure in the pipeline surfaces a user-visible notification — no silent failures
- **Dependencies:** Task 4.5, Task 6.2, Task 6.3
- **Acceptance criteria:** Errors surface as OS-native toast notifications; no silent failures

## 6. Data Model Changes

No database is used. All persistence is file-based:

- **Settings file:** `~/.pisum-langue/settings.json` (user home folder, same path on both platforms)

  ```json
  {
    "hotkey": { "key": "F9", "modifiers": [] },
    "audioFormat": "Opus",
    "providers": [
      { "name": "Google Primary", "type": "Google", "apiKey": "key-1", "enabled": true },
      { "name": "Google Secondary", "type": "Google", "apiKey": "key-2", "enabled": true }
    ],
    "prompt": "",
    "maxRecordingDurationSeconds": 600
  }
  ```

## 7. API Changes

No API endpoints are exposed. The app is a standalone desktop application.

**External API consumed:**

- Google Cloud Speech-to-Text API v2
  - Endpoint: `speech.googleapis.com`
  - Auth: API key or service account
  - Request: audio bytes + recognition config (encoding, sample rate, language, prompt)
  - Response: transcription text with confidence score

## 8. Dependencies & Risks

### External Dependencies (NuGet Packages)

| Package                                      | Purpose                                          | Phase |
| -------------------------------------------- | ------------------------------------------------ | ----- |
| `Avalonia`                                   | Cross-platform UI framework                      | 1     |
| `Avalonia.Desktop`                           | Desktop platform support                         | 1     |
| `Avalonia.Themes.Fluent`                     | Fluent design theme                              | 1     |
| `CommunityToolkit.Mvvm`                      | MVVM helpers (ObservableObject, RelayCommand)    | 5     |
| `NAudio`                                     | Windows audio capture                            | 2     |
| `Concentus` + `Concentus.OggFile`            | Opus encoding (cross-platform)                   | 2     |
| `Google.Cloud.Speech.V2`                     | Google Speech-to-Text SDK                        | 3     |
| `Microsoft.Extensions.DependencyInjection`   | DI container                                     | 1     |
| `Microsoft.Extensions.Logging`               | Logging abstractions                             | 1     |

### Risks & Mitigations

| Risk                                                     | Impact                               | Mitigation                                                                  |
| -------------------------------------------------------- | ------------------------------------ | --------------------------------------------------------------------------- |
| Global hotkey conflicts with other apps                  | Hotkey doesn't register              | Allow configurable hotkey; show error if registration fails                 |
| Audio device not available or changes mid-recording      | Recording fails                      | Detect device changes; show user notification                               |
| Google API latency exceeds 3s target                     | Poor UX                              | Show progress indicator; consider streaming in future                       |
| Avalonia TrayIcon behavior varies across platforms       | UX inconsistency                     | Test early on both platforms; use platform-specific fallbacks               |
| macOS permission prompts (Accessibility, Microphone)     | App doesn't work until approved      | Clear onboarding guidance; detect permission state                          |
| No internet / API unreachable                            | Transcription fails silently         | Detect network errors; show OS-native toast notification with clear message |

### Assumptions

- User has a working microphone configured as default audio input
- User has internet connectivity for cloud transcription
- User has a valid Google API key with Speech-to-Text enabled
- .NET 10 SDK is available for development

## 9. Testing Strategy

### Unit Tests

- **TranscriptionOrchestrator:** Mock all dependencies, test state machine (idle → recording → transcribing → pasting → idle), error paths trigger notification service
- **RoundRobinTranscriptionService:** Test cycling across providers, fallback on failure, empty provider list, single provider, settings change rebuilds list
- **SettingsService:** Test load/save/defaults/migration, verify file at `~/.pisum-langue/settings.json`
- **GoogleTranscriptionService:** Mock HTTP responses, test parsing and error handling
- **KeyCombination:** Test equality, serialization

### Integration Tests

- **Audio pipeline:** Record real audio (short burst) → encode → verify file format headers
- **Clipboard write:** Set text → verify text appears on OS clipboard
- **Settings persistence:** Save → restart service → load → verify values

### Manual E2E Test Scenarios

1. Fresh install: app starts, shows tray icon, default settings work
2. Configure hotkey → hold it → speak → release → text appears at cursor in Notepad
3. Configure hotkey → hold it → speak → release → text appears at cursor in browser text field
4. Configure hotkey → hold it → speak → release → text appears at cursor in VS Code
5. Record for >10 minutes → verify recording auto-stops
6. Disconnect internet → attempt transcription → verify OS toast notification with error message
7. Invalid API key → attempt transcription → verify OS toast notification with error message
8. Configure two providers → dictate multiple times → verify requests alternate between them
9. Disable one provider → verify all requests go to the remaining one
10. Repeat tests 2-4 on macOS

### Edge Cases

- Very short recording (tap and release hotkey immediately)
- Very long recording (near 10-minute limit)
- Rapid key press/release
- No microphone connected
- No internet connectivity when transcription is attempted
- Multiple monitors / different DPI scaling (for overlay indicator)
- All providers disabled or removed from settings
- Provider fails mid-round-robin (network error on one, success on fallback)

## 10. Requirement Traceability

### Functional Requirements

| PRD Ref          | Requirement Summary                               | Task(s)           | Notes                                                      |
| ---------------- | ------------------------------------------------- | ----------------- | ---------------------------------------------------------- |
| Recording #1     | Configurable global hotkey on Windows and macOS    | 2.1, 2.2, 5.3    | Both platforms from Phase 2                                |
| Recording #2     | Capture audio from default microphone              | 2.3, 2.4          | Both platforms from Phase 2                                |
| Recording #3     | Encode to compressed format (Opus, MP3)            | 2.5               | Opus preferred; M4A removed                                |
| Recording #4     | Stop recording on hotkey release (push-to-talk)    | 2.1, 2.2, 2.6    | Push-to-talk only                                          |
| Recording #5     | Audio/visual feedback for recording state          | 6.1               |                                                            |
| Transcription #1 | Send audio + prompt to AI provider                 | 3.1, 3.2, 3.3     | Round-robin across configured providers                    |
| Transcription #2 | Configurable prompts                               | 3.1, 5.3          | Prompt stored in settings                                  |
| Transcription #3 | AI provider behind ITranscriptionService           | 1.2, 3.1, 3.2     | Round-robin wrapper implements the interface                |
| Output #1        | Copy transcription to clipboard                    | 4.1, 4.3, 4.5    | Both platforms                                             |
| Output #2        | Simulate paste (Ctrl+V / Cmd+V)                    | 4.2, 4.4, 4.5    | Both platforms                                             |
| Error Handling #1| Show OS-native toast on errors                     | 6.2, 6.3, 6.4    | Windows toast + macOS notification                         |
| Error Handling #2| Detect network/API failures                        | 4.5, 6.4          | No silent failures                                         |
| Configuration #1 | Settings UI from system tray                       | 5.3               |                                                            |
| Configuration #2 | Persist settings between sessions                  | 5.1               |                                                            |
| Configuration #3 | Start minimized to tray / menu bar                 | 1.4               |                                                            |

### User Stories

| PRD Ref | User Story Summary                             | Implementing Tasks              | Fully Covered? |
| ------- | ---------------------------------------------- | ------------------------------- | -------------- |
| US-1    | Hold hotkey to dictate without switching apps   | 2.1, 2.2, 2.3, 2.4, 2.6, 3.3, 4.5 | Yes            |
| US-2    | Transcribed text appears at cursor              | 4.2, 4.4, 4.5                  | Yes            |
| US-3    | Transcribed text copied to clipboard            | 4.1, 4.3, 4.5                  | Yes            |
| US-4    | Configure AI provider and model                 | 5.1, 5.2, 5.3                  | Yes            |

### Success Metrics

| Metric                                        | How the Plan Addresses It                                                                                             |
| --------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| End-to-end latency < 3s for < 15s speech      | Opus compression minimizes upload size; orchestrator runs sequentially with no unnecessary delays; tested in E2E       |
| Works across common applications               | Manual E2E tests cover Notepad, browser, VS Code, chat apps on both platforms                                         |
| Swapping AI provider = new class + DI change   | `ITranscriptionService` abstraction (Task 1.2); Google implementation is just one registered class (Task 3.1)         |
| Minimal resource usage when idle               | No main window; tray-only; no polling; hotkey is event-driven                                                         |
