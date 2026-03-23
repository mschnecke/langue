# PRD: File-Based Logging with Retention

## 1. Introduction/Overview

Pisum Transcript currently has no logging infrastructure. Errors are shown to users as OS notifications, but there is no persistent record of application behavior, warnings, or errors. This makes debugging, support, and development significantly harder.

This feature adds structured file-based logging using the `tracing` crate, with configurable log levels, file rotation by size, automatic cleanup of old logs, and a new Logging tab in the Settings UI for user control.

## 2. Goals

- Provide persistent, structured log output to files for debugging and support purposes
- Implement automatic log rotation (by file size) and retention (by age) to prevent unbounded disk usage
- Allow users to configure logging behavior (log level, retention settings) through the Settings UI
- Replace ad-hoc error handling with consistent `tracing` instrumentation across all backend modules
- Make it easy for users to locate and share log files when reporting issues

## 3. User Stories

- As a **user experiencing issues**, I want application logs written to a file so that I can share them when reporting a bug.
- As a **developer debugging a problem**, I want to set the log level to DEBUG or TRACE so that I can see detailed internal behavior without rebuilding the app.
- As a **user with limited disk space**, I want logs to automatically rotate and old logs to be deleted so that they don't consume excessive storage.
- As a **user**, I want a button in Settings to open the log folder so that I can quickly find log files.
- As a **developer**, I want structured log output with timestamps, levels, and module context so that I can efficiently search and filter logs.

## 4. Functional Requirements

### 4.1 Logging Library Integration

1. The system must use the `tracing` crate as the logging facade across all Rust backend modules.
2. The system must use `tracing-appender` (or equivalent) to write logs to files.
3. The system must use `tracing-subscriber` to configure log formatting and filtering.
4. The system must initialize the logging subsystem early in application startup (before other subsystems).

### 4.2 Log Output

5. The system must write log entries to files in a dedicated logs directory under the user's home folder (e.g., `~/.pisum-transcript/logs/`).
6. Each log entry must include: timestamp (ISO 8601, local time), log level, module/target path, and the log message.
7. The system must support the standard five log levels: ERROR, WARN, INFO, DEBUG, TRACE.
8. The default log level must be INFO.
9. The system must also output logs to stdout/stderr when running in development mode (`tauri dev`).

### 4.3 Log Rotation and Retention

10. The system must rotate log files when a file exceeds a configurable maximum size (default: 1 MB).
11. Rotated files must be named with a timestamp or sequential suffix to preserve ordering.
12. The system must delete log files older than a configurable number of days (default: 7 days).
13. The retention cleanup must run at application startup.

### 4.4 Configuration

14. The following logging settings must be added to the application config schema:
    - `log_level`: one of `error`, `warn`, `info`, `debug`, `trace` (default: `info`)
    - `log_max_file_size_mb`: maximum log file size in MB before rotation (default: 10)
    - `log_retention_days`: number of days to keep log files (default: 7)
15. Changes to `log_level` should take effect immediately (without app restart) if feasible, or on next restart.
16. Changes to rotation/retention settings must take effect on next app restart.

### 4.5 Settings UI - Logging Tab

17. The Settings UI must include a new **Logging** tab accessible from the tab navigation.
18. The Logging tab must display a dropdown/select for **Log Level** (ERROR, WARN, INFO, DEBUG, TRACE).
19. The Logging tab must display a numeric input for **Max File Size** (in MB).
20. The Logging tab must display a numeric input for **Retention Period** (in days).
21. The Logging tab must include an **Open Log Folder** button that opens the logs directory in the system file explorer.
22. The Logging tab must show the current log file path as read-only informational text.

### 4.6 Instrumentation

23. All existing modules (`audio`, `ai`, `hotkey`, `output`, `config`, `tray`) must use `tracing` macros (`info!`, `warn!`, `error!`, `debug!`, `trace!`) instead of silent error propagation where appropriate.
24. Key application lifecycle events must be logged at INFO level: app startup, config loaded, hotkey registered, recording started/stopped, transcription started/completed, provider selected.
25. All errors that currently produce OS notifications must also be logged at ERROR level with full context.
26. Performance-sensitive paths (audio recording callbacks) must use TRACE level to avoid impacting performance at default log levels.

## 5. Non-Goals (Out of Scope)

- **Remote/cloud log shipping**: Logs are local only. No telemetry or remote collection.
- **Frontend (Svelte) logging**: This PRD covers Rust backend logging only. Frontend console logging is not in scope.
- **Log viewer UI**: The Settings UI provides a button to open the log folder, but does not include an in-app log viewer or log search.
- **Log encryption or redaction**: Logs are stored as plain text. Sensitive data redaction is not in scope.
- **Custom log format configuration**: The log format is fixed (not user-configurable).

## 6. Design Considerations

### Logging Tab Layout

The Logging tab in Settings should follow the same layout patterns as existing tabs (General, Audio, Provider, etc.):

- Section header: "Logging"
- Log Level: dropdown select with options ERROR / WARN / INFO / DEBUG / TRACE
- Max File Size: numeric input with "MB" suffix label
- Retention Period: numeric input with "days" suffix label
- Log File Location: read-only text showing the path
- Open Log Folder: button aligned with existing button styles

## 7. Technical Considerations

### Dependencies

- `tracing` — logging facade (macros: `info!`, `warn!`, `error!`, `debug!`, `trace!`)
- `tracing-subscriber` — subscriber configuration, formatting, filtering
- `tracing-appender` — file appender with rotation support

### Log File Location

Store logs in a `logs/` subdirectory under the user's home folder:
- Windows: `%USERPROFILE%/.pisum-transcript/logs/`
- macOS: `~/.pisum-transcript/logs/`
- Linux: `~/.pisum-transcript/logs/`

This is consistent with the existing config file location (`~/.pisum-transcript.json`).

### Integration Points

- **Config module** (`src-tauri/src/config/`): Add new logging fields to the config schema and manager.
- **App initialization** (`src-tauri/src/lib.rs`): Initialize tracing subscriber in `run()` before other setup.
- **Hotkey manager** (`src-tauri/src/hotkey/manager.rs`): Add tracing instrumentation alongside existing notification logic.
- **Tray module** (`src-tauri/src/tray.rs`): Expose an IPC command to open the log folder.
- **Frontend** (`src/`): Add Logging tab component and wire up IPC for settings and open-folder action.

### Constraints

- The `tracing` global subscriber can only be set once. If dynamic log level changes are needed, use a `reload` layer from `tracing-subscriber`.
- File I/O in the logging layer must be non-blocking or handled on a dedicated thread to avoid impacting audio recording performance.

## 8. Success Metrics

- All backend modules produce structured log output to files at appropriate levels.
- Log files are automatically rotated when exceeding the configured size limit.
- Log files older than the retention period are automatically cleaned up on startup.
- Users can change log level and retention settings from the Settings UI Logging tab.
- Users can locate log files via the Open Log Folder button.
- No measurable performance regression in audio recording or transcription latency at the default INFO log level.

## 9. Open Questions

- [x] Should `tracing-appender`'s built-in rotation be used, or a custom rotation implementation for more control over size-based rotation and age-based cleanup? -> built-in
- [x] Should the log level change take effect immediately via a `reload` layer, or require an app restart? (Immediate is better UX but adds complexity.) -> immediate
- [x] Should log output include span context (e.g., request IDs for transcription calls), or just flat key-value fields? -> flat
