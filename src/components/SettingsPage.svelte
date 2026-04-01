<script lang="ts">
	import type { AppSettings } from '$lib/types';
	import { persistSettings } from '$stores/settings';
	import { getVersion } from '@tauri-apps/api/app';
	import HotkeyConfig from './HotkeyConfig.svelte';
	import AudioConfig from './AudioConfig.svelte';
	import ProviderConfig from './ProviderConfig.svelte';
	import PresetConfig from './PresetConfig.svelte';
	import GeneralConfig from './GeneralConfig.svelte';
	import LoggingConfig from './LoggingConfig.svelte';
	import ModeToggle from './ModeToggle.svelte';
	import WhisperConfig from './WhisperConfig.svelte';

	import appIconUrl from '../../src-tauri/icons/icon.svg';

	let { settings }: { settings: AppSettings } = $props();

	let activeTab = $state<'general' | 'hotkey' | 'audio' | 'transcription' | 'presets' | 'logging'>(
		'transcription',
	);
	let appVersion = $state('');

	getVersion().then((v) => (appVersion = v));

	async function handleUpdate(updated: AppSettings) {
		await persistSettings(updated);
	}

	const tabs = [
		{ id: 'transcription' as const, label: 'Transcription' },
		{ id: 'presets' as const, label: 'Presets' },
		{ id: 'hotkey' as const, label: 'Hotkey' },
		{ id: 'audio' as const, label: 'Audio' },
		{ id: 'logging' as const, label: 'Logging' },
		{ id: 'general' as const, label: 'General' },
	];
</script>

<div class="flex h-screen flex-col">
	<!-- Header -->
	<div class="flex items-center gap-3 px-6 pb-0 pt-5">
		<img src={appIconUrl} alt="Pisum Transcript" class="h-8 w-8 rounded-lg" />
		<h1 class="text-lg font-semibold text-gray-900">Pisum Transcript Settings</h1>
	</div>

	<!-- Tabs -->
	<div class="px-6 pt-3">
		<nav class="flex gap-1 border-b border-gray-200">
			{#each tabs as tab (tab.id)}
				<button
					class="border-b-2 px-3 py-2 text-sm font-medium transition-colors {activeTab === tab.id
						? 'border-blue-500 text-blue-600'
						: 'border-transparent text-gray-500 hover:border-gray-300 hover:text-gray-700'}"
					onclick={() => (activeTab = tab.id)}
				>
					{tab.label}
				</button>
			{/each}
		</nav>
	</div>

	<!-- Tab content -->
	<div class="flex-1 overflow-y-auto px-6 py-4">
		{#if activeTab === 'transcription'}
			<div class="space-y-4">
				<ModeToggle {settings} onUpdate={handleUpdate} />
				{#if settings.transcriptionMode === 'local'}
					<WhisperConfig {settings} onUpdate={handleUpdate} />
				{:else}
					<ProviderConfig {settings} onUpdate={handleUpdate} />
				{/if}
			</div>
		{:else if activeTab === 'presets'}
			<PresetConfig {settings} onUpdate={handleUpdate} />
		{:else if activeTab === 'hotkey'}
			<HotkeyConfig {settings} onUpdate={handleUpdate} />
		{:else if activeTab === 'audio'}
			<AudioConfig {settings} onUpdate={handleUpdate} />
		{:else if activeTab === 'logging'}
			<LoggingConfig {settings} onUpdate={handleUpdate} />
		{:else if activeTab === 'general'}
			<GeneralConfig {settings} onUpdate={handleUpdate} />
		{/if}
	</div>

	<!-- Footer with version -->
	{#if appVersion}
		<div class="border-t border-gray-100 px-6 py-2 text-right text-xs text-gray-400">
			v{appVersion}
		</div>
	{/if}
</div>
