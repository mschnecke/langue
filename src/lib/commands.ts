import { invoke } from '@tauri-apps/api/core';
import type { AppSettings, HotkeyBinding, ModelInfo, Preset, ProviderConfig } from './types';

// Settings
export async function loadSettings(): Promise<AppSettings> {
  return invoke('load_settings');
}

export async function saveSettings(settings: AppSettings): Promise<void> {
  return invoke('save_settings', { settings });
}

// Presets
export async function getPresets(): Promise<Preset[]> {
  return invoke('get_presets');
}

export async function setActivePreset(presetId: string): Promise<void> {
  return invoke('set_active_preset', { presetId });
}

export async function savePreset(preset: Preset): Promise<void> {
  return invoke('save_preset', { preset });
}

export async function deletePreset(presetId: string): Promise<void> {
  return invoke('delete_preset', { presetId });
}

// Providers
export async function testProviderConnection(provider: ProviderConfig): Promise<boolean> {
  return invoke('test_provider_connection', { provider });
}

export async function listProviderModels(
  providerType: string,
  apiKey: string,
): Promise<ModelInfo[]> {
  return invoke('list_provider_models', { providerType, apiKey });
}

// Hotkeys
export async function registerHotkey(binding: HotkeyBinding): Promise<void> {
  return invoke('register_hotkey', { binding });
}

export async function unregisterHotkey(): Promise<void> {
  return invoke('unregister_hotkey');
}

export async function checkConflict(binding: HotkeyBinding): Promise<boolean> {
  return invoke('check_conflict', { binding });
}

export async function checkSystemConflict(binding: HotkeyBinding): Promise<boolean> {
  return invoke('check_system_conflict', { binding });
}

// Auto-start
export async function setAutostart(enabled: boolean): Promise<void> {
  return invoke('set_autostart', { enabled });
}
