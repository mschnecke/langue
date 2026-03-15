<script lang="ts">
  import type { AppSettings } from '$lib/types';
  import { setAutostart } from '$lib/commands';

  let { settings, onUpdate }: { settings: AppSettings; onUpdate: (s: AppSettings) => void } =
    $props();

  async function toggleStartWithSystem() {
    const newValue = !settings.startWithSystem;
    try {
      await setAutostart(newValue);
      onUpdate({ ...settings, startWithSystem: newValue });
    } catch (e) {
      console.error('Failed to set autostart:', e);
    }
  }

  function toggleNotifications() {
    onUpdate({ ...settings, showTrayNotifications: !settings.showTrayNotifications });
  }

  function setRecordingMode(mode: 'holdToRecord' | 'toggle') {
    onUpdate({ ...settings, recordingMode: mode });
  }

  function setMaxDuration(e: Event) {
    const input = e.target as HTMLInputElement;
    let value = parseInt(input.value, 10);
    if (isNaN(value) || value < 10) value = 10;
    if (value > 3600) value = 3600;
    onUpdate({ ...settings, maxRecordingDurationSecs: value });
  }
</script>

<div class="space-y-4">
  <h2 class="text-sm font-semibold text-gray-700 uppercase tracking-wide">General</h2>

  <label class="flex items-center justify-between py-2">
    <div>
      <p class="text-sm font-medium text-gray-900">Start with system</p>
      <p class="text-xs text-gray-500">Launch Pisum Transcript when you log in</p>
    </div>
    <button
      class="relative w-10 h-5 rounded-full transition-colors {settings.startWithSystem
        ? 'bg-blue-500'
        : 'bg-gray-300'}"
      onclick={toggleStartWithSystem}
      title="Toggle start with system"
    >
      <span
        class="absolute top-0.5 left-0.5 w-4 h-4 bg-white rounded-full shadow transition-transform {settings.startWithSystem
          ? 'translate-x-5'
          : ''}"
      ></span>
    </button>
  </label>

  <label class="flex items-center justify-between py-2">
    <div>
      <p class="text-sm font-medium text-gray-900">Show notifications</p>
      <p class="text-xs text-gray-500">Display OS notifications for errors and status</p>
    </div>
    <button
      class="relative w-10 h-5 rounded-full transition-colors {settings.showTrayNotifications
        ? 'bg-blue-500'
        : 'bg-gray-300'}"
      onclick={toggleNotifications}
      title="Toggle notifications"
    >
      <span
        class="absolute top-0.5 left-0.5 w-4 h-4 bg-white rounded-full shadow transition-transform {settings.showTrayNotifications
          ? 'translate-x-5'
          : ''}"
      ></span>
    </button>
  </label>

  <div class="pt-2">
    <h3 class="text-sm font-medium text-gray-900 mb-1">Recording Mode</h3>
    <p class="text-xs text-gray-500 mb-3">
      Choose how the hotkey controls recording.
    </p>
    <div class="flex gap-3">
      <button
        class="flex-1 px-4 py-3 rounded-lg border-2 text-sm font-medium transition-colors {settings.recordingMode ===
        'holdToRecord'
          ? 'border-blue-500 bg-blue-50 text-blue-700'
          : 'border-gray-200 text-gray-600 hover:border-gray-300'}"
        onclick={() => setRecordingMode('holdToRecord')}
      >
        <div class="font-semibold">Hold to Record</div>
        <div class="text-xs mt-1 {settings.recordingMode === 'holdToRecord' ? 'text-blue-500' : 'text-gray-400'}">
          Hold the hotkey to record. Release to transcribe and paste.
        </div>
      </button>

      <button
        class="flex-1 px-4 py-3 rounded-lg border-2 text-sm font-medium transition-colors {settings.recordingMode ===
        'toggle'
          ? 'border-blue-500 bg-blue-50 text-blue-700'
          : 'border-gray-200 text-gray-600 hover:border-gray-300'}"
        onclick={() => setRecordingMode('toggle')}
      >
        <div class="font-semibold">Toggle (Start/Stop)</div>
        <div class="text-xs mt-1 {settings.recordingMode === 'toggle' ? 'text-blue-500' : 'text-gray-400'}">
          Press the hotkey to start recording. Press again to transcribe and paste.
        </div>
      </button>
    </div>
  </div>

  <label class="block pt-2">
    <h3 class="text-sm font-medium text-gray-900 mb-1">Max Recording Duration</h3>
    <p class="text-xs text-gray-500 mb-2">
      Recording auto-stops after this duration.
    </p>
    <div class="flex items-center gap-2">
      <input
        type="number"
        min="10"
        max="3600"
        value={settings.maxRecordingDurationSecs}
        onchange={setMaxDuration}
        class="w-24 px-3 py-2 border border-gray-200 rounded-lg text-sm"
      />
      <span class="text-xs text-gray-500">seconds</span>
    </div>
  </label>
</div>
