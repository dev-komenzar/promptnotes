<script lang="ts">
	import { toastStore } from '../stores/toasts.svelte';

	type Props = {
		store?: typeof toastStore;
	};

	let { store = toastStore }: Props = $props();

	const entries = $derived(store.entries);
</script>

<!--
	Toast region (画面下部中央, 縦パイル / 新しい順に上積み)。
	widget-update-toast (右下) とは表示位置が排他なので衝突しない。
-->
<aside
	data-testid="region-toast"
	aria-label="Deletion toast stack"
	aria-live="polite"
	class="pointer-events-none fixed inset-x-0 bottom-4 z-20 flex flex-col items-center gap-2"
>
	<div data-testid="screen-1-toast-stack" class="flex w-full max-w-sm flex-col-reverse gap-2 px-2">
		{#each entries as entry (entry.id)}
			<div
				data-testid="screen-1-toast"
				data-toast-id={entry.id}
				role="status"
				class="pointer-events-auto flex items-center gap-2 rounded-md border border-neutral-200 bg-white px-3 py-2 text-sm shadow-md dark:border-neutral-700 dark:bg-neutral-900"
			>
				<span
					data-testid="screen-1-toast-message"
					class="min-w-0 flex-1 truncate text-neutral-700 dark:text-neutral-200"
					title={entry.preview}
				>
					Deleted: {entry.preview}
				</span>
				<button
					type="button"
					data-testid="screen-1-toast-undo"
					data-toast-id={entry.id}
					class="shrink-0 rounded border border-blue-200 px-2 py-0.5 text-xs text-blue-600 hover:bg-blue-50 dark:border-blue-700 dark:text-blue-300 dark:hover:bg-blue-900/20"
					aria-label="Undo"
					onclick={() => {
						void store.undo(entry.id);
					}}>Undo</button
				>
				<button
					type="button"
					data-testid="screen-1-toast-close"
					data-toast-id={entry.id}
					class="shrink-0 rounded px-1.5 text-xs text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200"
					aria-label="Close toast"
					onclick={() => store.dismiss(entry.id)}>×</button
				>
			</div>
		{/each}
	</div>
</aside>
