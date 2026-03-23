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
    <h2 class="text-sm font-semibold text-gray-700 uppercase tracking-wide">Whisper Engine</h2>
    {#if status.state === 'ready'}
      <span class="text-xs bg-green-100 text-green-700 px-2 py-0.5 rounded-full">Ready</span>
    {:else if status.state === 'noModel'}
      <span class="text-xs bg-yellow-100 text-yellow-700 px-2 py-0.5 rounded-full"
        >No model loaded</span
      >
    {:else}
      <span class="text-xs bg-gray-100 text-gray-500 px-2 py-0.5 rounded-full">Inactive</span>
    {/if}
  </div>

  {#if error}
    <div class="bg-red-50 border border-red-200 rounded-lg px-4 py-3">
      <p class="text-sm text-red-700">{error}</p>
    </div>
  {/if}

  <!-- Input Language -->
  <div>
    <label for="whisper-language" class="block text-xs font-medium text-gray-600 mb-1"
      >Input Language</label
    >
    <select
      id="whisper-language"
      value={settings.whisperConfig.language}
      onchange={(e) => updateWhisperConfig({ language: e.currentTarget.value as WhisperLanguage })}
      class="w-full px-3 py-1.5 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-300 focus:border-blue-400 bg-white"
    >
      <option value="auto">Auto-detect</option>
      <option value="german">German</option>
      <option value="english">English</option>
    </select>
    <p class="text-xs text-gray-400 mt-1">Language of the spoken audio input.</p>
  </div>

  <!-- Translate to English -->
  <div class="flex items-center justify-between">
    <div>
      <p class="text-sm font-medium text-gray-700">Translate to English</p>
      <p class="text-xs text-gray-400">Translate non-English speech into English output.</p>
    </div>
    <button
      class="relative w-9 h-5 rounded-full transition-colors {settings.whisperConfig
        .translateToEnglish
        ? 'bg-blue-500'
        : 'bg-gray-300'}"
      title={settings.whisperConfig.translateToEnglish ? 'Disable translation' : 'Enable translation'}
      onclick={() =>
        updateWhisperConfig({ translateToEnglish: !settings.whisperConfig.translateToEnglish })}
    >
      <span
        class="absolute top-0.5 left-0.5 w-4 h-4 bg-white rounded-full shadow transition-transform {settings
          .whisperConfig.translateToEnglish
          ? 'translate-x-4'
          : ''}"
      ></span>
    </button>
  </div>

  <!-- Download Progress -->
  {#if downloading && downloadProgress}
    <div class="bg-blue-50 border border-blue-200 rounded-lg px-4 py-3 space-y-2">
      <div class="flex items-center justify-between">
        <p class="text-sm text-blue-700 font-medium">
          Downloading... {downloadProgress.percentage.toFixed(1)}%
        </p>
        <button
          class="text-xs text-blue-600 hover:text-blue-800 font-medium"
          onclick={handleCancel}
        >
          Cancel
        </button>
      </div>
      <div class="w-full bg-blue-200 rounded-full h-2">
        <div
          class="bg-blue-500 h-2 rounded-full transition-all"
          style="width: {downloadProgress.percentage}%"
        ></div>
      </div>
      <p class="text-xs text-blue-600">
        {formatBytes(downloadProgress.bytesDownloaded)} / {formatBytes(
          downloadProgress.totalBytes,
        )}
      </p>
    </div>
  {/if}

  <!-- Models -->
  <div>
    <p class="text-xs font-medium text-gray-600 mb-2">Models</p>
    <div class="space-y-2">
      {#each models as model (model.id)}
        <div
          class="border rounded-lg px-4 py-3 {model.downloaded &&
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
                <span class="text-xs bg-green-100 text-green-700 px-2 py-0.5 rounded"
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
                  class="px-3 py-1 text-xs font-medium bg-blue-500 text-white rounded-lg hover:bg-blue-600 transition-colors disabled:opacity-50"
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
    <div class="bg-yellow-50 border border-yellow-200 rounded-lg px-4 py-3">
      <p class="text-sm text-yellow-700 font-medium">No model downloaded</p>
      <p class="text-xs text-yellow-600 mt-1">
        Download a model above to enable local transcription.
      </p>
    </div>
  {/if}
</div>
