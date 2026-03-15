<script lang="ts">
  import type { AppSettings } from '$lib/types';
  import { persistSettings } from '$stores/settings';
  import HotkeyConfig from './HotkeyConfig.svelte';
  import AudioConfig from './AudioConfig.svelte';
  import ProviderConfig from './ProviderConfig.svelte';
  import PresetConfig from './PresetConfig.svelte';
  import GeneralConfig from './GeneralConfig.svelte';

  let { settings }: { settings: AppSettings } = $props();

  let activeTab = $state<'general' | 'hotkey' | 'audio' | 'providers' | 'presets'>('providers');

  async function handleUpdate(updated: AppSettings) {
    await persistSettings(updated);
  }

  const tabs = [
    { id: 'providers' as const, label: 'Providers' },
    { id: 'presets' as const, label: 'Presets' },
    { id: 'hotkey' as const, label: 'Hotkey' },
    { id: 'audio' as const, label: 'Audio' },
    { id: 'general' as const, label: 'General' },
  ];
</script>

<div class="flex flex-col h-screen">
  <!-- Header -->
  <div class="px-6 pt-5 pb-0">
    <h1 class="text-lg font-semibold text-gray-900">Pisum Transcript Settings</h1>
  </div>

  <!-- Tabs -->
  <div class="px-6 pt-3">
    <nav class="flex gap-1 border-b border-gray-200">
      {#each tabs as tab}
        <button
          class="px-3 py-2 text-sm font-medium border-b-2 transition-colors {activeTab === tab.id
            ? 'border-blue-500 text-blue-600'
            : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'}"
          onclick={() => (activeTab = tab.id)}
        >
          {tab.label}
        </button>
      {/each}
    </nav>
  </div>

  <!-- Tab content -->
  <div class="flex-1 overflow-y-auto px-6 py-4">
    {#if activeTab === 'providers'}
      <ProviderConfig {settings} onUpdate={handleUpdate} />
    {:else if activeTab === 'presets'}
      <PresetConfig {settings} onUpdate={handleUpdate} />
    {:else if activeTab === 'hotkey'}
      <HotkeyConfig {settings} onUpdate={handleUpdate} />
    {:else if activeTab === 'audio'}
      <AudioConfig {settings} onUpdate={handleUpdate} />
    {:else if activeTab === 'general'}
      <GeneralConfig {settings} onUpdate={handleUpdate} />
    {/if}
  </div>
</div>
