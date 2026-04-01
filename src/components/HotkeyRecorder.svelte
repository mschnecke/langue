<script lang="ts">
	import { onMount } from 'svelte';
	import { SvelteSet } from 'svelte/reactivity';
	import type { HotkeyBinding } from '$lib/types';

	let {
		onCapture,
		onCancel,
	}: { onCapture: (binding: HotkeyBinding) => void; onCancel: () => void } = $props();

	let activeModifiers = new SvelteSet<string>();
	let capturedKey = $state<string | null>(null);
	let element: HTMLDivElement | undefined = $state();

	const MODIFIER_KEYS: Record<string, string> = {
		Control: 'Ctrl',
		Shift: 'Shift',
		Alt: 'Alt',
		Meta: 'Cmd',
	};

	function mapKeyName(key: string): string {
		if (key.length === 1) return key.toUpperCase();
		if (key.startsWith('Arrow')) return key.slice(5);
		if (key === ' ') return 'Space';
		return key;
	}

	function handleKeyDown(e: KeyboardEvent) {
		e.preventDefault();
		e.stopPropagation();

		if (e.key === 'Escape') {
			onCancel();
			return;
		}

		if (e.key in MODIFIER_KEYS) {
			activeModifiers.add(MODIFIER_KEYS[e.key]);
			return;
		}

		// Non-modifier key: require at least one modifier
		if (activeModifiers.size === 0) return;

		capturedKey = mapKeyName(e.key);
		const binding: HotkeyBinding = {
			modifiers: [...activeModifiers],
			key: capturedKey,
		};
		onCapture(binding);
	}

	function handleKeyUp(e: KeyboardEvent) {
		e.preventDefault();
		if (e.key in MODIFIER_KEYS) {
			activeModifiers.delete(MODIFIER_KEYS[e.key]);
		}
	}

	onMount(() => {
		element?.focus();
	});
</script>

<div
	bind:this={element}
	class="rounded-lg border-2 border-dashed border-blue-300 bg-blue-50 px-4 py-4 text-center focus:outline-none"
	role="textbox"
	tabindex="0"
	onkeydown={handleKeyDown}
	onkeyup={handleKeyUp}
>
	<p class="text-sm font-medium text-blue-700">
		{#if activeModifiers.size > 0}
			{[...activeModifiers].join(' + ')} + ...
		{:else}
			Press a key combination...
		{/if}
	</p>
	<p class="mt-1 text-xs text-blue-500">Hold modifier(s) then press a key. Escape to cancel.</p>
</div>
