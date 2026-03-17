<script lang="ts">
  import type { AppSettings, Preset } from '$lib/types';
  import { setActivePreset, savePreset, deletePreset } from '$lib/commands';
  import { initSettings } from '$stores/settings';

  let { settings, onUpdate }: { settings: AppSettings; onUpdate: (s: AppSettings) => void } =
    $props();

  let editingId = $state<string | null>(null);
  let editName = $state('');
  let editPrompt = $state('');
  let adding = $state(false);
  let newName = $state('');
  let newPrompt = $state('');

  async function handleSetActive(presetId: string) {
    try {
      await setActivePreset(presetId);
      await initSettings();
    } catch {
      // silently ignore
    }
  }

  function startEdit(preset: Preset) {
    editingId = preset.id;
    editName = preset.name;
    editPrompt = preset.systemPrompt;
    adding = false;
  }

  async function handleSaveEdit() {
    if (!editingId || !editName.trim()) return;
    try {
      await savePreset({
        id: editingId,
        name: editName.trim(),
        systemPrompt: editPrompt.trim(),
        isBuiltin: settings.presets.find((p) => p.id === editingId)?.isBuiltin ?? false,
      });
      editingId = null;
      await initSettings();
    } catch {
      // silently ignore
    }
  }

  async function handleDelete(presetId: string) {
    try {
      await deletePreset(presetId);
      await initSettings();
    } catch {
      // silently ignore
    }
  }

  function startAdd() {
    adding = true;
    editingId = null;
    newName = '';
    newPrompt = '';
  }

  async function handleAdd() {
    if (!newName.trim() || !newPrompt.trim()) return;
    try {
      await savePreset({
        id: crypto.randomUUID(),
        name: newName.trim(),
        systemPrompt: newPrompt.trim(),
        isBuiltin: false,
      });
      adding = false;
      await initSettings();
    } catch {
      // silently ignore
    }
  }
</script>

<div class="space-y-4">
  <div class="flex items-center justify-between">
    <h2 class="text-sm font-semibold text-gray-700 uppercase tracking-wide">Prompt Presets</h2>
    <button
      class="px-3 py-1.5 bg-blue-500 text-white text-xs font-medium rounded-lg hover:bg-blue-600 transition-colors"
      onclick={startAdd}
    >
      + Add Preset
    </button>
  </div>

  <p class="text-xs text-gray-500">
    Presets define the system prompt sent to the AI. The active preset determines the transcription
    language and behavior.
  </p>

  <!-- Add new preset form -->
  {#if adding}
    <div class="border-2 border-blue-200 rounded-lg p-4 space-y-3 bg-blue-50">
      <input
        type="text"
        bind:value={newName}
        placeholder="Preset name..."
        class="w-full px-3 py-1.5 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-300"
      />
      <textarea
        bind:value={newPrompt}
        placeholder="System prompt..."
        rows="3"
        class="w-full px-3 py-1.5 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-300 resize-y"
      ></textarea>
      <div class="flex gap-2">
        <button
          class="px-3 py-1.5 bg-blue-500 text-white text-xs font-medium rounded-lg hover:bg-blue-600 disabled:opacity-50"
          onclick={handleAdd}
          disabled={!newName.trim() || !newPrompt.trim()}
        >
          Save
        </button>
        <button
          class="px-3 py-1.5 text-xs text-gray-600 border border-gray-200 rounded-lg hover:bg-gray-50"
          onclick={() => (adding = false)}
        >
          Cancel
        </button>
      </div>
    </div>
  {/if}

  <!-- Preset list -->
  {#each settings.presets as preset (preset.id)}
    <div
      class="border rounded-lg p-4 transition-colors {preset.id === settings.activePresetId
        ? 'border-blue-300 bg-blue-50'
        : 'border-gray-200'}"
    >
      {#if editingId === preset.id}
        <!-- Edit mode -->
        <div class="space-y-3">
          <input
            type="text"
            bind:value={editName}
            class="w-full px-3 py-1.5 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-300"
          />
          <textarea
            bind:value={editPrompt}
            rows="3"
            class="w-full px-3 py-1.5 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-300 resize-y"
          ></textarea>
          <div class="flex gap-2">
            <button
              class="px-3 py-1.5 bg-blue-500 text-white text-xs font-medium rounded-lg hover:bg-blue-600"
              onclick={handleSaveEdit}
            >
              Save
            </button>
            <button
              class="px-3 py-1.5 text-xs text-gray-600 border border-gray-200 rounded-lg hover:bg-gray-50"
              onclick={() => (editingId = null)}
            >
              Cancel
            </button>
          </div>
        </div>
      {:else}
        <!-- Display mode -->
        <div class="flex items-start justify-between">
          <div class="flex-1">
            <div class="flex items-center gap-2">
              <span class="text-sm font-medium text-gray-900">{preset.name}</span>
              {#if preset.isBuiltin}
                <span class="text-[10px] bg-gray-100 text-gray-500 px-1.5 py-0.5 rounded"
                  >Built-in</span
                >
              {/if}
              {#if preset.id === settings.activePresetId}
                <span class="text-[10px] bg-blue-100 text-blue-600 px-1.5 py-0.5 rounded font-medium"
                  >Active</span
                >
              {/if}
            </div>
            <p class="text-xs text-gray-500 mt-1 line-clamp-2">{preset.systemPrompt}</p>
          </div>
          <div class="flex items-center gap-1 ml-3">
            {#if preset.id !== settings.activePresetId}
              <button
                class="px-2 py-1 text-xs text-blue-600 hover:bg-blue-50 rounded"
                onclick={() => handleSetActive(preset.id)}
              >
                Activate
              </button>
            {/if}
            <button
              class="px-2 py-1 text-xs text-gray-500 hover:bg-gray-50 rounded"
              onclick={() => startEdit(preset)}
            >
              Edit
            </button>
            {#if !preset.isBuiltin}
              <button
                class="px-2 py-1 text-xs text-red-500 hover:bg-red-50 rounded"
                onclick={() => handleDelete(preset.id)}
              >
                Delete
              </button>
            {/if}
          </div>
        </div>
      {/if}
    </div>
  {/each}
</div>
