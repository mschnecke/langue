<script lang="ts">
	import type { AppSettings, Preset } from '$lib/types';
	import { setActivePreset, savePreset, deletePreset } from '$lib/commands';
	import { initSettings } from '$stores/settings';

	let {
		settings,
		onUpdate: _onUpdate,
	}: { settings: AppSettings; onUpdate: (s: AppSettings) => void } = $props();

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
		<h2 class="text-sm font-semibold uppercase tracking-wide text-gray-700">Prompt Presets</h2>
		<button
			class="rounded-lg bg-blue-500 px-3 py-1.5 text-xs font-medium text-white transition-colors hover:bg-blue-600"
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
		<div class="space-y-3 rounded-lg border-2 border-blue-200 bg-blue-50 p-4">
			<input
				type="text"
				bind:value={newName}
				placeholder="Preset name..."
				class="w-full rounded-lg border border-gray-200 px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-blue-300"
			/>
			<textarea
				bind:value={newPrompt}
				placeholder="System prompt..."
				rows="3"
				class="w-full resize-y rounded-lg border border-gray-200 px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-blue-300"
			></textarea>
			<div class="flex gap-2">
				<button
					class="rounded-lg bg-blue-500 px-3 py-1.5 text-xs font-medium text-white hover:bg-blue-600 disabled:opacity-50"
					onclick={handleAdd}
					disabled={!newName.trim() || !newPrompt.trim()}
				>
					Save
				</button>
				<button
					class="rounded-lg border border-gray-200 px-3 py-1.5 text-xs text-gray-600 hover:bg-gray-50"
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
			class="rounded-lg border p-4 transition-colors {preset.id === settings.activePresetId
				? 'border-blue-300 bg-blue-50'
				: 'border-gray-200'}"
		>
			{#if editingId === preset.id}
				<!-- Edit mode -->
				<div class="space-y-3">
					<input
						type="text"
						bind:value={editName}
						class="w-full rounded-lg border border-gray-200 px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-blue-300"
					/>
					<textarea
						bind:value={editPrompt}
						rows="3"
						class="w-full resize-y rounded-lg border border-gray-200 px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-blue-300"
					></textarea>
					<div class="flex gap-2">
						<button
							class="rounded-lg bg-blue-500 px-3 py-1.5 text-xs font-medium text-white hover:bg-blue-600"
							onclick={handleSaveEdit}
						>
							Save
						</button>
						<button
							class="rounded-lg border border-gray-200 px-3 py-1.5 text-xs text-gray-600 hover:bg-gray-50"
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
								<span class="rounded bg-gray-100 px-1.5 py-0.5 text-[10px] text-gray-500"
									>Built-in</span
								>
							{/if}
							{#if preset.id === settings.activePresetId}
								<span
									class="rounded bg-blue-100 px-1.5 py-0.5 text-[10px] font-medium text-blue-600"
									>Active</span
								>
							{/if}
						</div>
						<p class="mt-1 line-clamp-2 text-xs text-gray-500">{preset.systemPrompt}</p>
					</div>
					<div class="ml-3 flex items-center gap-1">
						{#if preset.id !== settings.activePresetId}
							<button
								class="rounded px-2 py-1 text-xs text-blue-600 hover:bg-blue-50"
								onclick={() => handleSetActive(preset.id)}
							>
								Activate
							</button>
						{/if}
						<button
							class="rounded px-2 py-1 text-xs text-gray-500 hover:bg-gray-50"
							onclick={() => startEdit(preset)}
						>
							Edit
						</button>
						{#if !preset.isBuiltin}
							<button
								class="rounded px-2 py-1 text-xs text-red-500 hover:bg-red-50"
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
