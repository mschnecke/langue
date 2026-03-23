export interface AppSettings {
  startWithSystem: boolean;
  showTrayNotifications: boolean;
  hotkey: HotkeyBinding;
  audioFormat: 'opus' | 'wav';
  presets: Preset[];
  activePresetId: string;
  providers: ProviderConfig[];
  recordingMode: 'holdToRecord' | 'toggle';
  maxRecordingDurationSecs: number;
  transcriptionMode: TranscriptionMode;
  whisperConfig: WhisperConfig;
  loggingConfig: LoggingConfig;
}

export interface Preset {
  id: string;
  name: string;
  systemPrompt: string;
  isBuiltin: boolean;
}

export interface ProviderConfig {
  id: string;
  providerType: 'gemini' | 'openai';
  apiKey: string;
  model: string | null;
  enabled: boolean;
}

export interface ModelInfo {
  id: string;
  displayName: string;
}

export interface HotkeyBinding {
  modifiers: string[];
  key: string;
}

export type TranscriptionMode = 'cloud' | 'local';
export type WhisperLanguage = 'auto' | 'german' | 'english';

export interface WhisperConfig {
  selectedModel: string;
  language: WhisperLanguage;
  translateToEnglish: boolean;
}

export interface LoggingConfig {
  logLevel: 'error' | 'warn' | 'info' | 'debug' | 'trace';
  logMaxFileSizeMb: number;
  logRetentionDays: number;
}

export interface WhisperModelInfo {
  id: string;
  name: string;
  fileName: string;
  sizeBytes: number;
  description: string;
  downloaded: boolean;
  fileSizeOnDisk: number | null;
}

export interface WhisperStatus {
  state: 'ready' | 'loading' | 'noModel' | 'notActive';
  loadedModel: string | null;
}

export interface DownloadProgress {
  modelId: string;
  bytesDownloaded: number;
  totalBytes: number;
  percentage: number;
}
