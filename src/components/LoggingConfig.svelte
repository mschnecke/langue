<script lang="ts">
  import type { AppSettings } from '$lib/types';
  import { openLogFolder, getLogPath } from '$lib/commands';

  let { settings, onUpdate }: { settings: AppSettings; onUpdate: (s: AppSettings) => void } =
    $props();

  let logPath = $state('');

  $effect(() => {
    getLogPath().then((p) => (logPath = p));
  });

  function setLogLevel(e: Event) {
    const value = (e.target as HTMLSelectElement).value;
    onUpdate({
      ...settings,
      loggingConfig: { ...settings.loggingConfig, logLevel: value as AppSettings['loggingConfig']['logLevel'] },
    });
  }

  function setMaxFileSize(e: Event) {
    const input = e.target as HTMLInputElement;
    let value = parseInt(input.value, 10);
    if (isNaN(value) || value < 1) value = 1;
    if (value > 100) value = 100;
    onUpdate({
      ...settings,
      loggingConfig: { ...settings.loggingConfig, logMaxFileSizeMb: value },
    });
  }

  function setRetentionDays(e: Event) {
    const input = e.target as HTMLInputElement;
    let value = parseInt(input.value, 10);
    if (isNaN(value) || value < 1) value = 1;
    if (value > 365) value = 365;
    onUpdate({
      ...settings,
      loggingConfig: { ...settings.loggingConfig, logRetentionDays: value },
    });
  }

  async function handleOpenLogFolder() {
    try {
      await openLogFolder();
    } catch {
      // silently ignore
    }
  }
</script>

<div class="space-y-4">
  <h2 class="text-sm font-semibold text-gray-700 uppercase tracking-wide">Logging</h2>

  <label class="block">
    <h3 class="text-sm font-medium text-gray-900 mb-1">Log Level</h3>
    <p class="text-xs text-gray-500 mb-2">
      Controls the verbosity of log output. Changes take effect immediately.
    </p>
    <select
      value={settings.loggingConfig.logLevel}
      onchange={setLogLevel}
      class="w-full px-3 py-2 border border-gray-200 rounded-lg text-sm"
    >
      <option value="error">Error</option>
      <option value="warn">Warn</option>
      <option value="info">Info (default)</option>
      <option value="debug">Debug</option>
      <option value="trace">Trace (most verbose)</option>
    </select>
  </label>

  <label class="block">
    <h3 class="text-sm font-medium text-gray-900 mb-1">Max Log File Size</h3>
    <p class="text-xs text-gray-500 mb-2">
      Log files are rotated when they exceed this size. Takes effect on restart.
    </p>
    <div class="flex items-center gap-2">
      <input
        type="number"
        min="1"
        max="100"
        value={settings.loggingConfig.logMaxFileSizeMb}
        onchange={setMaxFileSize}
        class="w-24 px-3 py-2 border border-gray-200 rounded-lg text-sm"
      />
      <span class="text-xs text-gray-500">MB</span>
    </div>
  </label>

  <label class="block">
    <h3 class="text-sm font-medium text-gray-900 mb-1">Retention Period</h3>
    <p class="text-xs text-gray-500 mb-2">
      Log files older than this are deleted on startup.
    </p>
    <div class="flex items-center gap-2">
      <input
        type="number"
        min="1"
        max="365"
        value={settings.loggingConfig.logRetentionDays}
        onchange={setRetentionDays}
        class="w-24 px-3 py-2 border border-gray-200 rounded-lg text-sm"
      />
      <span class="text-xs text-gray-500">days</span>
    </div>
  </label>

  {#if logPath}
    <div class="pt-2">
      <h3 class="text-sm font-medium text-gray-900 mb-1">Log File Location</h3>
      <p class="text-xs text-gray-500 break-all">{logPath}</p>
    </div>
  {/if}

  <button
    onclick={handleOpenLogFolder}
    class="px-4 py-2 text-sm font-medium text-blue-600 bg-blue-50 rounded-lg hover:bg-blue-100 transition-colors"
  >
    Open Log Folder
  </button>
</div>
