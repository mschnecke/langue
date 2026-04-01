<script lang="ts">
	import type { AppSettings } from '$lib/types';

	let { settings, onUpdate }: { settings: AppSettings; onUpdate: (s: AppSettings) => void } =
		$props();

	function setFormat(format: 'opus' | 'wav') {
		onUpdate({ ...settings, audioFormat: format });
	}
</script>

<div class="space-y-4">
	<h2 class="text-sm font-semibold uppercase tracking-wide text-gray-700">Audio Format</h2>
	<p class="text-xs text-gray-500">
		Choose the audio encoding format sent to the AI provider. Opus is smaller and faster; WAV is
		uncompressed.
	</p>

	<div class="flex gap-3">
		<button
			class="flex-1 rounded-lg border-2 px-4 py-3 text-sm font-medium transition-colors {settings.audioFormat ===
			'opus'
				? 'border-blue-500 bg-blue-50 text-blue-700'
				: 'border-gray-200 text-gray-600 hover:border-gray-300'}"
			onclick={() => setFormat('opus')}
		>
			<div class="font-semibold">Opus</div>
			<div
				class="mt-1 text-xs {settings.audioFormat === 'opus' ? 'text-blue-500' : 'text-gray-400'}"
			>
				Compressed, recommended
			</div>
		</button>

		<button
			class="flex-1 rounded-lg border-2 px-4 py-3 text-sm font-medium transition-colors {settings.audioFormat ===
			'wav'
				? 'border-blue-500 bg-blue-50 text-blue-700'
				: 'border-gray-200 text-gray-600 hover:border-gray-300'}"
			onclick={() => setFormat('wav')}
		>
			<div class="font-semibold">WAV</div>
			<div
				class="mt-1 text-xs {settings.audioFormat === 'wav' ? 'text-blue-500' : 'text-gray-400'}"
			>
				Uncompressed, larger files
			</div>
		</button>
	</div>
</div>
