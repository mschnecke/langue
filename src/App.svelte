<script lang="ts">
	import { onMount } from 'svelte';
	import { settings, settingsError, initSettings } from '$stores/settings';
	import SettingsPage from '$components/SettingsPage.svelte';

	let loading = $state(true);

	onMount(async () => {
		await initSettings();
		loading = false;
	});
</script>

<main class="no-select min-h-screen bg-white text-gray-900">
	{#if loading}
		<div class="flex h-screen items-center justify-center">
			<p class="text-gray-400">Loading settings...</p>
		</div>
	{:else if $settingsError}
		<div class="p-6">
			<div class="rounded-lg border border-red-200 bg-red-50 p-4">
				<p class="font-medium text-red-700">Failed to load settings</p>
				<p class="mt-1 text-sm text-red-600">{$settingsError}</p>
			</div>
		</div>
	{:else if $settings}
		<SettingsPage settings={$settings} />
	{/if}
</main>
