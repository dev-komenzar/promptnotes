<script lang="ts">
	import { untrack } from 'svelte';
	import {
		createUpdateToastStore,
		type UpdateEventListener,
		type UpdateInstallableFn,
		type UpdateInstalledHook,
		type UpdateInstallFn,
		type UpdateSubscribedHook,
		type UpdateUrlOpener
	} from './store.svelte';

	type Props = {
		listenFn?: UpdateEventListener;
		openUrlFn?: UpdateUrlOpener;
		/** listen 確立後に 1 回呼ばれる。page-main が起動時 check-for-updates を流す。 */
		onSubscribed?: UpdateSubscribedHook;
		installableFn?: UpdateInstallableFn;
		installFn?: UpdateInstallFn;
		/** install 成功後に 1 回呼ばれる。page-main が全 Note を Flush してから再起動する (I-U8)。 */
		onInstalled?: UpdateInstalledHook;
	};

	let { listenFn, openUrlFn, onSubscribed, installableFn, installFn, onInstalled }: Props =
		$props();

	const store = untrack(() =>
		createUpdateToastStore({
			listenFn,
			openUrlFn,
			onSubscribed,
			installableFn,
			installFn,
			onInstalled
		})
	);

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

	function handleInstall() {
		void store.install();
	}
</script>

{#if store.payload}
	{@const p = store.payload}
	<div
		data-testid="widget.widget-update-toast.root"
		role="status"
		aria-live="polite"
		class="pointer-events-none fixed right-4 bottom-4 z-50"
	>
		<div
			class="pointer-events-auto flex w-80 max-w-[90vw] flex-col gap-2 rounded-lg border border-neutral-200 bg-white p-3 text-sm shadow-lg dark:border-neutral-700 dark:bg-neutral-900 dark:text-neutral-100"
		>
			<div class="flex items-start justify-between gap-2">
				<p>
					New version
					<span data-testid="widget.widget-update-toast.latest-version" class="font-semibold"
						>{p.latest_version}</span
					>
					is available
					<span
						data-testid="widget.widget-update-toast.current-version"
						class="text-xs text-neutral-500 dark:text-neutral-400"
					>
						(current: {p.current_version})
					</span>
				</p>
				<button
					type="button"
					data-testid="widget.widget-update-toast.dismiss"
					aria-label="Close"
					disabled={store.installStatus === 'installing'}
					class="-mt-0.5 -mr-0.5 shrink-0 rounded p-1 text-neutral-500 hover:bg-neutral-100 hover:text-neutral-900 dark:hover:bg-neutral-800 dark:hover:text-neutral-100"
					onclick={handleDismiss}
				>
					×
				</button>
			</div>

			{#if p.release_notes.trim().length > 0}
				<p
					data-testid="widget.widget-update-toast.release-notes-summary"
					class="line-clamp-2 text-xs text-neutral-600 dark:text-neutral-400"
				>
					{truncate(p.release_notes)}
				</p>
			{/if}

			{#if store.installStatus === 'installing'}
				<p
					data-testid="widget.widget-update-toast.install-status"
					class="text-xs text-neutral-600 dark:text-neutral-400"
				>
					Updating…
				</p>
			{:else if store.installStatus === 'failed'}
				<p
					data-testid="widget.widget-update-toast.install-status"
					role="alert"
					class="text-xs text-red-600 dark:text-red-400"
				>
					Update failed. Try again or download it from the release page.
				</p>
			{:else if store.installStatus === 'already-installing'}
				<p
					data-testid="widget.widget-update-toast.install-status"
					class="text-xs text-neutral-600 dark:text-neutral-400"
				>
					An update is already being installed.
				</p>
			{:else if store.installStatus === 'restart-required'}
				<p
					data-testid="widget.widget-update-toast.install-status"
					role="alert"
					class="text-xs text-neutral-600 dark:text-neutral-400"
				>
					The update was installed. Restart PromptNotes to finish updating.
				</p>
			{/if}

			<div class="flex justify-end gap-1">
				<button
					type="button"
					data-testid="widget.widget-update-toast.view-release"
					class="rounded px-2 py-1 text-xs font-medium text-blue-600 hover:bg-blue-50 dark:text-blue-400 dark:hover:bg-blue-950"
					onclick={handleViewRelease}
				>
					View release
				</button>
				{#if store.installable}
					<button
						type="button"
						data-testid="widget.widget-update-toast.install-update"
						disabled={store.installStatus === 'installing' ||
							store.installStatus === 'restart-required'}
						class="rounded bg-blue-600 px-2 py-1 text-xs font-medium text-white hover:bg-blue-700 disabled:cursor-not-allowed disabled:opacity-60 dark:bg-blue-500 dark:hover:bg-blue-600"
						onclick={handleInstall}
					>
						Update & restart
					</button>
				{/if}
			</div>
		</div>
	</div>
{/if}
