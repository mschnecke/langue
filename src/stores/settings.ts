import { writable } from 'svelte/store';
import type { AppSettings } from '$lib/types';
import { loadSettings, saveSettings as saveSettingsCmd } from '$lib/commands';

export const settings = writable<AppSettings | null>(null);
export const settingsError = writable<string | null>(null);

export async function initSettings(): Promise<void> {
	try {
		const loaded = await loadSettings();
		settings.set(loaded);
		settingsError.set(null);
	} catch (e) {
		settingsError.set(String(e));
	}
}

export async function persistSettings(updated: AppSettings): Promise<void> {
	try {
		await saveSettingsCmd(updated);
		settings.set(updated);
		settingsError.set(null);
	} catch (e) {
		settingsError.set(String(e));
	}
}
