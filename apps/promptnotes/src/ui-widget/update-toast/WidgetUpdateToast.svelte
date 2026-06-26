<script lang="ts">
	import { untrack } from 'svelte';
	import {
		createUpdateToastStore,
		type UpdateEventListener,
		type UpdateUrlOpener
	} from './store.svelte';

	type Props = {
		listenFn?: UpdateEventListener;
		openUrlFn?: UpdateUrlOpener;
	};

	let { listenFn, openUrlFn }: Props = $props();

	const store = untrack(() => createUpdateToastStore({ listenFn, openUrlFn }));

	$effect(() => {
		void store.start();
		return () => store.stop();
	});

	const RELEASE_NOTES_MAX = 140;

	function truncate(notes: string): string {
		const trimmed = notes.trim();
		if (trimmed.length <= RELEASE_NOTES_MAX) return trimmed;
		return trimmed.slice(0, RELEASE_NOTES_MAX).trimEnd() + '…';
	}

	function handleViewRelease() {
		void store.viewRelease();
	}

	function handleDismiss() {
		store.dismiss();
	}
</script>

{#if store.payload}
	{@const p = store.payload}
	<div
		data-testid="widget-update-toast"
		role="status"
		aria-live="polite"
		class="pointer-events-none fixed right-4 bottom-4 z-50"
	>
		<div
			class="pointer-events-auto flex w-80 max-w-[90vw] flex-col gap-2 rounded-lg border border-neutral-200 bg-white p-3 text-sm shadow-lg dark:border-neutral-700 dark:bg-neutral-900 dark:text-neutral-100"
		>
			<div class="flex items-start justify-between gap-2">
				<p>
					新しいバージョン
					<span data-testid="screen-3-latest-version" class="font-semibold">{p.latest_version}</span
					>
					が利用可能です
					<span
						data-testid="screen-3-current-version"
						class="text-xs text-neutral-500 dark:text-neutral-400"
					>
						(現在: {p.current_version})
					</span>
				</p>
				<button
					type="button"
					data-testid="screen-3-dismiss"
					aria-label="閉じる"
					class="-mt-0.5 -mr-0.5 shrink-0 rounded p-1 text-neutral-500 hover:bg-neutral-100 hover:text-neutral-900 dark:hover:bg-neutral-800 dark:hover:text-neutral-100"
					onclick={handleDismiss}
				>
					×
				</button>
			</div>

			{#if p.release_notes.trim().length > 0}
				<p
					data-testid="screen-3-release-notes-summary"
					class="line-clamp-2 text-xs text-neutral-600 dark:text-neutral-400"
				>
					{truncate(p.release_notes)}
				</p>
			{/if}

			<div class="flex justify-end">
				<button
					type="button"
					data-testid="screen-3-view-release"
					class="rounded px-2 py-1 text-xs font-medium text-blue-600 hover:bg-blue-50 dark:text-blue-400 dark:hover:bg-blue-950"
					onclick={handleViewRelease}
				>
					詳細を見る
				</button>
			</div>
		</div>
	</div>
{/if}
