<script lang="ts">
  import type { AppSettings, TranscriptionMode } from '$lib/types';

  let { settings, onUpdate }: { settings: AppSettings; onUpdate: (s: AppSettings) => void } =
    $props();

  function setMode(mode: TranscriptionMode) {
    if (settings.transcriptionMode !== mode) {
      onUpdate({ ...settings, transcriptionMode: mode });
    }
  }
</script>

<div class="flex rounded-lg border border-gray-200 overflow-hidden">
  <button
    class="flex-1 px-4 py-2 text-sm font-medium transition-colors {settings.transcriptionMode === 'cloud'
      ? 'bg-blue-500 text-white'
      : 'bg-white text-gray-600 hover:bg-gray-50'}"
    onclick={() => setMode('cloud')}
  >
    Cloud (API)
  </button>
  <button
    class="flex-1 px-4 py-2 text-sm font-medium transition-colors {settings.transcriptionMode === 'local'
      ? 'bg-blue-500 text-white'
      : 'bg-white text-gray-600 hover:bg-gray-50'}"
    onclick={() => setMode('local')}
  >
    Local (Whisper)
  </button>
</div>
