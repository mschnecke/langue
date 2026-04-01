<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { listen } from '@tauri-apps/api/event';
	import type {
		AppSettings,
		WhisperModelInfo,
		WhisperStatus,
		WhisperLanguage,
		DownloadProgress,
	} from '$lib/types';
	import {
		getAvailableModels,
		downloadWhisperModel,
		cancelWhisperDownload,
		deleteWhisperModel,
		getWhisperStatus,
	} from '$lib/commands';

	let { settings, onUpdate }: { settings: AppSettings; onUpdate: (s: AppSettings) => void } =
		$props();

	let models = $state<WhisperModelInfo[]>([]);
	let status = $state<WhisperStatus>({ state: 'notActive', loadedModel: null });
	let downloadProgress = $state<DownloadProgress | null>(null);
	let downloading = $state(false);
	let error = $state<string | null>(null);
	let loaded = $state(false);

	let unlisten: (() => void) | null = null;

	onMount(async () => {
		await refreshData();
		loaded = true;

		const unlistenFn = await listen<DownloadProgress>('whisper-download-progress', (event) => {
			downloadProgress = event.payload;
		});
		unlisten = unlistenFn;
	});

	onDestroy(() => {
		unlisten?.();
	});

	async function refreshData() {
		try {
			[models, status] = await Promise.all([getAvailableModels(), getWhisperStatus()]);
		} catch (e) {
			error = String(e);
		}
	}

	async function handleDownload(modelId: string) {
		downloading = true;
		downloadProgress = null;
		error = null;
		try {
			await downloadWhisperModel(modelId);
		} catch (e) {
			error = String(e);
		} finally {
			downloading = false;
			downloadProgress = null;
			await refreshData();
		}
	}

	async function handleCancel() {
		try {
			await cancelWhisperDownload();
		} catch (e) {
			error = String(e);
		}
	}

	async function handleDelete(modelId: string) {
		error = null;
		try {
			await deleteWhisperModel(modelId);
			await refreshData();
		} catch (e) {
			error = String(e);
		}
	}

	function updateWhisperConfig(updates: Partial<typeof settings.whisperConfig>) {
		onUpdate({
			...settings,
			whisperConfig: { ...settings.whisperConfig, ...updates },
		});
	}

	function formatBytes(bytes: number): string {
		if (bytes >= 1_000_000_000) return `${(bytes / 1_000_000_000).toFixed(1)} GB`;
		if (bytes >= 1_000_000) return `${(bytes / 1_000_000).toFixed(0)} MB`;
		return `${(bytes / 1_000).toFixed(0)} KB`;
	}
</script>

<div class="space-y-4">
	<!-- Status -->
	<div class="flex items-center gap-2">
		<h2 class="text-sm font-semibold uppercase tracking-wide text-gray-700">Whisper Engine</h2>
		{#if status.state === 'ready'}
			<span class="rounded-full bg-green-100 px-2 py-0.5 text-xs text-green-700">Ready</span>
		{:else if status.state === 'noModel'}
			<span class="rounded-full bg-yellow-100 px-2 py-0.5 text-xs text-yellow-700"
				>No model loaded</span
			>
		{:else}
			<span class="rounded-full bg-gray-100 px-2 py-0.5 text-xs text-gray-500">Inactive</span>
		{/if}
	</div>

	{#if error}
		<div class="rounded-lg border border-red-200 bg-red-50 px-4 py-3">
			<p class="text-sm text-red-700">{error}</p>
		</div>
	{/if}

	<!-- Input Language -->
	<div>
		<label for="whisper-language" class="mb-1 block text-xs font-medium text-gray-600"
			>Input Language</label
		>
		<select
			id="whisper-language"
			value={settings.whisperConfig.language}
			onchange={(e) => updateWhisperConfig({ language: e.currentTarget.value as WhisperLanguage })}
			class="w-full rounded-lg border border-gray-200 bg-white px-3 py-1.5 text-sm focus:border-blue-400 focus:outline-none focus:ring-2 focus:ring-blue-300"
		>
			<option value="auto">Auto-detect</option>
			<option value="german">German</option>
			<option value="english">English</option>
		</select>
		<p class="mt-1 text-xs text-gray-400">Language of the spoken audio input.</p>
	</div>

	<!-- Translate to English -->
	<div class="flex items-center justify-between">
		<div>
			<p class="text-sm font-medium text-gray-700">Translate to English</p>
			<p class="text-xs text-gray-400">Translate non-English speech into English output.</p>
		</div>
		<button
			class="relative h-5 w-9 rounded-full transition-colors {settings.whisperConfig
				.translateToEnglish
				? 'bg-blue-500'
				: 'bg-gray-300'}"
			title={settings.whisperConfig.translateToEnglish
				? 'Disable translation'
				: 'Enable translation'}
			onclick={() =>
				updateWhisperConfig({ translateToEnglish: !settings.whisperConfig.translateToEnglish })}
		>
			<span
				class="absolute left-0.5 top-0.5 h-4 w-4 rounded-full bg-white shadow transition-transform {settings
					.whisperConfig.translateToEnglish
					? 'translate-x-4'
					: ''}"
			></span>
		</button>
	</div>

	<!-- Download Progress -->
	{#if downloading && downloadProgress}
		<div class="space-y-2 rounded-lg border border-blue-200 bg-blue-50 px-4 py-3">
			<div class="flex items-center justify-between">
				<p class="text-sm font-medium text-blue-700">
					Downloading... {downloadProgress.percentage.toFixed(1)}%
				</p>
				<button
					class="text-xs font-medium text-blue-600 hover:text-blue-800"
					onclick={handleCancel}
				>
					Cancel
				</button>
			</div>
			<div class="h-2 w-full rounded-full bg-blue-200">
				<div
					class="h-2 rounded-full bg-blue-500 transition-all"
					style="width: {downloadProgress.percentage}%"
				></div>
			</div>
			<p class="text-xs text-blue-600">
				{formatBytes(downloadProgress.bytesDownloaded)} / {formatBytes(downloadProgress.totalBytes)}
			</p>
		</div>
	{/if}

	<!-- Models -->
	<div>
		<p class="mb-2 text-xs font-medium text-gray-600">Models</p>
		<div class="space-y-2">
			{#each models as model (model.id)}
				<div
					class="rounded-lg border px-4 py-3 {model.downloaded &&
					settings.whisperConfig.selectedModel === model.id
						? 'border-blue-300 bg-blue-50'
						: 'border-gray-200'}"
				>
					<div class="flex items-center justify-between">
						<div class="flex items-center gap-2">
							{#if model.downloaded}
								<input
									type="radio"
									name="whisper-model"
									checked={settings.whisperConfig.selectedModel === model.id}
									onchange={() => updateWhisperConfig({ selectedModel: model.id })}
									class="text-blue-500"
								/>
							{/if}
							<div>
								<p class="text-sm font-medium text-gray-900">{model.name}</p>
								<p class="text-xs text-gray-500">{model.description}</p>
							</div>
						</div>
						<div class="flex items-center gap-2">
							<span class="text-xs text-gray-400">{formatBytes(model.sizeBytes)}</span>
							{#if model.downloaded}
								<span class="rounded bg-green-100 px-2 py-0.5 text-xs text-green-700"
									>Downloaded</span
								>
								<button
									class="text-xs text-red-500 hover:text-red-700"
									onclick={() => handleDelete(model.id)}
								>
									Delete
								</button>
							{:else}
								<button
									class="rounded-lg bg-blue-500 px-3 py-1 text-xs font-medium text-white transition-colors hover:bg-blue-600 disabled:opacity-50"
									onclick={() => handleDownload(model.id)}
									disabled={downloading}
								>
									Download
								</button>
							{/if}
						</div>
					</div>
				</div>
			{/each}
		</div>
	</div>

	{#if loaded && models.length > 0 && models.every((m) => !m.downloaded)}
		<div class="rounded-lg border border-yellow-200 bg-yellow-50 px-4 py-3">
			<p class="text-sm font-medium text-yellow-700">No model downloaded</p>
			<p class="mt-1 text-xs text-yellow-600">
				Download a model above to enable local transcription.
			</p>
		</div>
	{/if}
</div>
