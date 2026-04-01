<script lang="ts">
	import type { AppSettings } from '$lib/types';
	import { checkSystemConflict } from '$lib/commands';
	import HotkeyRecorder from './HotkeyRecorder.svelte';
	import type { HotkeyBinding } from '$lib/types';

	let { settings, onUpdate }: { settings: AppSettings; onUpdate: (s: AppSettings) => void } =
		$props();

	let recording = $state(false);
	let conflictWarning = $state<string | null>(null);

	function formatHotkey(binding: HotkeyBinding): string {
		return [...binding.modifiers, binding.key].join(' + ');
	}

	async function handleNewBinding(binding: HotkeyBinding) {
		recording = false;
		conflictWarning = null;

		try {
			const conflicts = await checkSystemConflict(binding);
			if (conflicts) {
				conflictWarning = `${formatHotkey(binding)} conflicts with a system hotkey. It may not work reliably.`;
			}
		} catch {
			// Non-fatal, proceed anyway
		}

		onUpdate({ ...settings, hotkey: binding });
	}
</script>

<div class="space-y-4">
	<h2 class="text-sm font-semibold uppercase tracking-wide text-gray-700">Hotkey</h2>
	<p class="text-xs text-gray-500">
		{#if settings.recordingMode === 'toggle'}
			Press this key combination to start recording. Press again to transcribe and paste.
		{:else}
			Hold this key combination to record, release to transcribe and paste.
		{/if}
	</p>

	{#if recording}
		<HotkeyRecorder onCapture={handleNewBinding} onCancel={() => (recording = false)} />
	{:else}
		<div class="flex items-center gap-3">
			<div
				class="flex-1 rounded-lg border border-gray-200 bg-gray-50 px-4 py-2.5 font-mono text-sm"
			>
				{formatHotkey(settings.hotkey)}
			</div>
			<button
				class="rounded-lg bg-blue-500 px-4 py-2.5 text-sm font-medium text-white transition-colors hover:bg-blue-600"
				onclick={() => (recording = true)}
			>
				Change
			</button>
		</div>
	{/if}

	{#if conflictWarning}
		<div class="rounded-lg border border-yellow-200 bg-yellow-50 px-3 py-2 text-xs text-yellow-700">
			{conflictWarning}
		</div>
	{/if}
</div>
