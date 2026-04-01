<script lang="ts">
	import { onMount } from 'svelte';
	import type { AppSettings, ModelInfo, ProviderConfig as ProviderConfigType } from '$lib/types';
	import { testProviderConnection, listProviderModels } from '$lib/commands';

	let { settings, onUpdate }: { settings: AppSettings; onUpdate: (s: AppSettings) => void } =
		$props();

	onMount(() => {
		for (const provider of settings.providers) {
			if (provider.apiKey) {
				fetchModels(provider);
			}
		}
	});

	let testingId = $state<string | null>(null);
	let testResult = $state<{ id: string; success: boolean; message: string } | null>(null);
	let showApiKey = $state<Record<string, boolean>>({});
	let modelsCache = $state<Record<string, ModelInfo[]>>({});
	let loadingModels = $state<Record<string, boolean>>({});

	function addProvider() {
		const id = crypto.randomUUID();
		const newProvider: ProviderConfigType = {
			id,
			providerType: 'gemini',
			apiKey: '',
			model: null,
			enabled: true,
		};
		onUpdate({ ...settings, providers: [...settings.providers, newProvider] });
	}

	function removeProvider(id: string) {
		onUpdate({ ...settings, providers: settings.providers.filter((p) => p.id !== id) });
	}

	function updateProvider(id: string, updates: Partial<ProviderConfigType>) {
		const providers = settings.providers.map((p) => (p.id === id ? { ...p, ...updates } : p));
		onUpdate({ ...settings, providers });
	}

	async function testConnection(provider: ProviderConfigType) {
		if (!provider.apiKey) {
			testResult = { id: provider.id, success: false, message: 'API key is required' };
			return;
		}

		testingId = provider.id;
		testResult = null;

		try {
			const success = await testProviderConnection(provider);
			testResult = { id: provider.id, success, message: success ? 'Connected' : 'Failed' };
		} catch (e) {
			testResult = { id: provider.id, success: false, message: String(e) };
		} finally {
			testingId = null;
		}
	}

	function toggleShowKey(id: string) {
		showApiKey = { ...showApiKey, [id]: !showApiKey[id] };
	}

	async function fetchModels(provider: ProviderConfigType) {
		if (!provider.apiKey) return;

		const cacheKey = `${provider.providerType}:${provider.apiKey}`;
		if (modelsCache[cacheKey]) return;

		loadingModels = { ...loadingModels, [provider.id]: true };
		try {
			const models = await listProviderModels(provider.providerType, provider.apiKey);
			modelsCache = { ...modelsCache, [cacheKey]: models };
		} catch {
			// silently ignore
		} finally {
			loadingModels = { ...loadingModels, [provider.id]: false };
		}
	}

	function getModels(provider: ProviderConfigType): ModelInfo[] {
		const cacheKey = `${provider.providerType}:${provider.apiKey}`;
		return modelsCache[cacheKey] ?? [];
	}
</script>

<div class="space-y-4">
	<div class="flex items-center justify-between">
		<h2 class="text-sm font-semibold uppercase tracking-wide text-gray-700">AI Providers</h2>
		<button
			class="rounded-lg bg-blue-500 px-3 py-1.5 text-xs font-medium text-white transition-colors hover:bg-blue-600"
			onclick={addProvider}
		>
			+ Add Provider
		</button>
	</div>

	{#if settings.providers.length === 0}
		<div class="rounded-lg border border-yellow-200 bg-yellow-50 px-4 py-3">
			<p class="text-sm font-medium text-yellow-700">No providers configured</p>
			<p class="mt-1 text-xs text-yellow-600">
				Add an AI provider with an API key to enable transcription.
			</p>
		</div>
	{/if}

	{#each settings.providers as provider (provider.id)}
		<div class="space-y-3 rounded-lg border border-gray-200 p-4">
			<div class="flex items-center justify-between">
				<div class="flex items-center gap-2">
					<select
						value={provider.providerType}
						onchange={(e) => {
							const newType = e.currentTarget.value as 'gemini' | 'openai';
							// Clear model and cached models when switching provider type
							const oldCacheKey = `${provider.providerType}:${provider.apiKey}`;
							const { [oldCacheKey]: _, ...rest } = modelsCache;
							modelsCache = rest;
							updateProvider(provider.id, { providerType: newType, model: null });
						}}
						class="rounded-lg border border-gray-200 bg-white px-2 py-0.5 text-sm font-medium text-gray-900 focus:outline-none focus:ring-2 focus:ring-blue-300"
					>
						<option value="gemini">Gemini</option>
						<option value="openai">OpenAI</option>
					</select>
					{#if !provider.enabled}
						<span class="rounded bg-gray-100 px-2 py-0.5 text-xs text-gray-500">Disabled</span>
					{/if}
				</div>
				<div class="flex items-center gap-2">
					<button
						class="relative h-5 w-9 rounded-full transition-colors {provider.enabled
							? 'bg-blue-500'
							: 'bg-gray-300'}"
						onclick={() => updateProvider(provider.id, { enabled: !provider.enabled })}
						title={provider.enabled ? 'Disable' : 'Enable'}
					>
						<span
							class="absolute left-0.5 top-0.5 h-4 w-4 rounded-full bg-white shadow transition-transform {provider.enabled
								? 'translate-x-4'
								: ''}"
						></span>
					</button>
					<button
						class="text-xs text-red-500 hover:text-red-700"
						onclick={() => removeProvider(provider.id)}
					>
						Remove
					</button>
				</div>
			</div>

			<!-- API Key -->
			<div>
				<label for="apikey-{provider.id}" class="mb-1 block text-xs font-medium text-gray-600"
					>API Key</label
				>
				<div class="flex gap-2">
					<input
						id="apikey-{provider.id}"
						type={showApiKey[provider.id] ? 'text' : 'password'}
						value={provider.apiKey}
						oninput={(e) => updateProvider(provider.id, { apiKey: e.currentTarget.value })}
						placeholder="Enter API key..."
						class="flex-1 rounded-lg border border-gray-200 px-3 py-1.5 text-sm focus:border-blue-400 focus:outline-none focus:ring-2 focus:ring-blue-300"
					/>
					<button
						class="rounded-lg border border-gray-200 px-2 py-1.5 text-xs text-gray-500 hover:bg-gray-50"
						onclick={() => toggleShowKey(provider.id)}
					>
						{showApiKey[provider.id] ? 'Hide' : 'Show'}
					</button>
				</div>
			</div>

			<!-- Model -->
			<div>
				<label for="model-{provider.id}" class="mb-1 block text-xs font-medium text-gray-600"
					>Model</label
				>
				<div class="flex gap-2">
					<select
						id="model-{provider.id}"
						value={provider.model ?? ''}
						onchange={(e) =>
							updateProvider(provider.id, {
								model: e.currentTarget.value || null,
							})}
						onfocus={() => fetchModels(provider)}
						disabled={!provider.apiKey}
						class="flex-1 rounded-lg border border-gray-200 bg-white px-3 py-1.5 text-sm focus:border-blue-400 focus:outline-none focus:ring-2 focus:ring-blue-300 disabled:bg-gray-50 disabled:text-gray-400"
					>
						<option value=""
							>{provider.providerType === 'openai'
								? 'Default (gpt-4o-mini-audio-preview)'
								: 'Default (gemini-2.5-flash-lite)'}</option
						>
						{#if loadingModels[provider.id]}
							<option disabled>Loading models...</option>
						{/if}
						{#each getModels(provider) as model (model.id)}
							<option value={model.id}>{model.displayName} ({model.id})</option>
						{/each}
					</select>
					<button
						class="rounded-lg border border-gray-200 px-2 py-1.5 text-xs text-gray-500 hover:bg-gray-50 disabled:opacity-50"
						onclick={() => {
							const cacheKey = `${provider.providerType}:${provider.apiKey}`;
							const { [cacheKey]: _, ...rest } = modelsCache;
							modelsCache = rest;
							fetchModels(provider);
						}}
						disabled={!provider.apiKey || loadingModels[provider.id]}
						title="Refresh models"
					>
						{loadingModels[provider.id] ? '...' : 'Refresh'}
					</button>
				</div>
			</div>

			<!-- Test Connection -->
			<div class="flex items-center gap-3">
				<button
					class="rounded-lg border border-gray-200 px-3 py-1.5 text-xs font-medium transition-colors hover:bg-gray-50 disabled:opacity-50"
					onclick={() => testConnection(provider)}
					disabled={testingId === provider.id || !provider.apiKey}
				>
					{testingId === provider.id ? 'Testing...' : 'Test Connection'}
				</button>
				{#if testResult && testResult.id === provider.id}
					<span class="text-xs {testResult.success ? 'text-green-600' : 'text-red-600'}">
						{testResult.message}
					</span>
				{/if}
			</div>
		</div>
	{/each}
</div>
