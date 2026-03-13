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
}

export interface Preset {
  id: string;
  name: string;
  systemPrompt: string;
  isBuiltin: boolean;
}

export interface ProviderConfig {
  id: string;
  providerType: 'gemini';
  apiKey: string;
  model: string | null;
  enabled: boolean;
}

export interface HotkeyBinding {
  modifiers: string[];
  key: string;
}
