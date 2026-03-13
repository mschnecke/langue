# PRD: AI-Driven Transcription and Translation

## 1. Introduction/Overview

Pisum Langue is a desktop application that allows users to upload audio files, automatically transcribe the spoken content into text, and then translate that text into a selected target language. The application solves the problem of manual transcription and translation workflows by combining both steps into a single, AI-powered tool. The AI provider is abstracted behind an interface so the underlying service can be swapped without changing the rest of the application.

## 2. Goals

- Allow users to transcribe audio files (MP3, WAV, etc.) into text with high accuracy
- Allow users to translate transcribed text into one target language at a time
- Export results to common file formats (SRT, TXT, DOCX)
- Keep the AI provider abstracted so it can be replaced or upgraded independently
- Deliver a responsive, intuitive desktop experience

## 3. User Stories

1. As a user, I want to upload an audio file so that I can get a text transcription of its content.
2. As a user, I want to select a target language and translate my transcription so that I can understand content in a different language.
3. As a user, I want to export my transcription or translation to a file (TXT, SRT, or DOCX) so that I can use it outside the application.
4. As a user, I want to see the progress of transcription and translation so that I know how long the process will take.
5. As a user, I want to review and edit the transcribed text before translating so that I can correct any errors.

## 4. Functional Requirements

1. The system must allow users to upload audio files in MP3 and WAV formats.
2. The system must validate that uploaded files are supported audio formats before processing.
3. The system must send the uploaded audio to an AI transcription service and display the resulting text.
4. The system must show a progress indicator while transcription is in progress.
5. The system must allow users to select one target language from a list of supported languages.
6. The system must send the transcribed text to an AI translation service and display the translated text.
7. The system must allow users to edit the transcribed text before requesting a translation.
8. The system must allow users to export results (transcription or translation) to TXT format.
9. The system must allow users to export results to SRT subtitle format with timestamps.
10. The system must allow users to export results to DOCX format.
11. The system must display clear error messages when transcription or translation fails (e.g., unsupported file, service unavailable).
12. The AI provider (transcription and translation) must be abstracted behind an interface so that the implementation can be swapped without modifying consuming code.
13. The system must persist the selected target language as a user preference between sessions.

## 5. Non-Goals (Out of Scope)

- Not included: Live microphone recording or real-time transcription
- Not included: Video file support
- Not included: Translating into multiple languages simultaneously
- Not included: Auto-detection of the source language (user specifies or defaults to a configured language)
- Not included: Cloud-based web UI — this is a desktop application only
- Not included: User accounts, authentication, or multi-user support
- Not included: Storing past transcriptions in a database (results are exported to files)

## 6. Design Considerations

- The application is a desktop app built with .NET
- The main window should have a clear workflow: upload → transcribe → (edit) → translate → export
- Use a split-pane or tabbed layout to show transcription on one side and translation on the other
- The export button should offer a dropdown or dialog to choose the output format (TXT, SRT, DOCX)
- Progress indicators should be non-blocking so the user can still interact with the UI

## 7. Technical Considerations

- **AI Provider Abstraction:** Define an `ITranscriptionService` and `ITranslationService` interface. Concrete implementations can wrap Azure Cognitive Services, OpenAI Whisper, Google Cloud, or any other provider.
- **File Handling:** Audio files should be read from disk and streamed to the AI service. Large files may need chunking depending on provider limits.
- **SRT Export:** Transcription must include timestamp data (start/end times per segment) to support SRT export. The AI provider interface should return timestamped segments, not just plain text.
- **DOCX Export:** Use a library such as Open XML SDK or similar to generate DOCX files.
- **.NET Desktop Framework:** Consider WPF or WinUI for the desktop UI. The choice should be finalized during implementation planning.
- **Dependency Injection:** Use the built-in .NET DI container to register AI service implementations, making provider swaps a configuration change.

## 8. Success Metrics

- Users can transcribe an audio file and receive text output within a reasonable time relative to file length
- Users can translate transcribed text into a selected target language
- Users can successfully export results in all three formats (TXT, SRT, DOCX)
- Swapping the AI provider requires only adding a new implementation class and changing DI registration — no other code changes

## 9. Open Questions

- [ ] Which .NET desktop framework should be used — WPF or WinUI?
- [ ] What is the maximum audio file size or duration to support?
- [ ] Which languages should be available for translation in the initial release?
- [ ] Should the application support offline/local AI models (e.g., local Whisper) in addition to cloud APIs?
- [ ] What is the default source language assumption if auto-detection is out of scope?
