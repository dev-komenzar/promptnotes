<script lang="ts">
	import { untrack } from 'svelte';
	import { createConflictDialogStore } from './store.svelte';
	import type { ConflictDialogStoreDeps } from './store.svelte';

	type Props = {
		/** 現在編集中の Note 本文（表示用） */
		localBody: string;
		/** 親がダイアログを閉じる時の callback */
		onClose: () => void;
		/** store deps の上書き（テスト容易性） */
		deps?: ConflictDialogStoreDeps;
	};

	let { localBody, onClose, deps }: Props = $props();

	const store = untrack(() => createConflictDialogStore(deps));

	let dialogEl: HTMLDialogElement | null = $state(null);

	$effect(() => {
		store.start();
		return () => store.stop();
	});

	$effect(() => {
		if (store.state === 'compare') {
			dialogEl?.showModal();
		}
	});

	function handleApply() {
		store.apply();
		dialogEl?.close();
		onClose();
	}

	function handleCancel() {
		store.cancel();
		dialogEl?.close();
		onClose();
	}

	function handleDialogCancel() {
		store.cancel();
		dialogEl?.close();
		onClose();
	}
</script>

{#if store.conflictPayload}
	<dialog
		bind:this={dialogEl}
		data-testid="widget-external-change-conflict"
		aria-labelledby="widget-conflict-title"
		class="rounded-lg border border-neutral-200 bg-white p-0 text-neutral-900 shadow-xl backdrop:bg-transparent dark:border-neutral-800 dark:bg-neutral-900 dark:text-neutral-100"
		oncancel={handleDialogCancel}
	>
		<form method="dialog" class="flex w-[36rem] max-w-[90vw] flex-col gap-4 p-5">
			<h1
				id="widget-conflict-title"
				class="text-base font-semibold"
				data-testid="screen-4-note-title"
			>
				{store.conflictPayload.note_title}
			</h1>

			<p class="text-sm text-neutral-600 dark:text-neutral-400">
				"<span class="font-mono">{store.conflictPayload.note_title}</span>" が外部で変更されました。
				編集中の内容と競合しています。
			</p>

			<div class="grid grid-cols-2 gap-3">
				<section class="flex flex-col gap-1">
					<span class="text-xs font-medium text-neutral-500 dark:text-neutral-400">
						Your Version
					</span>
					<textarea
						data-testid="screen-4-body-local"
						readonly
						disabled
						value={localBody}
						rows={6}
						class="w-full resize-none rounded-md border border-neutral-200 bg-neutral-50 p-2 font-mono text-xs text-neutral-700 dark:border-neutral-700 dark:bg-neutral-800 dark:text-neutral-300"
					></textarea>
				</section>

				<section class="flex flex-col gap-1">
					<span class="text-xs font-medium text-neutral-500 dark:text-neutral-400">
						External Version
					</span>
					<textarea
						data-testid="screen-4-body-external"
						readonly
						disabled
						value={store.conflictPayload.note_body}
						rows={6}
						class="w-full resize-none rounded-md border border-neutral-200 bg-neutral-50 p-2 font-mono text-xs text-neutral-700 dark:border-neutral-700 dark:bg-neutral-800 dark:text-neutral-300"
					></textarea>
				</section>
			</div>

			<fieldset data-testid="screen-4-resolution-choice" class="flex flex-col gap-1.5">
				<legend class="text-sm font-medium">解決方法</legend>
				<label class="flex items-center gap-2 text-sm">
					<input
						type="radio"
						name="conflict-resolution"
						value="ApplyExternal"
						data-testid="screen-4-resolution-apply-external"
						checked={store.resolution === 'ApplyExternal'}
						onchange={() => store.selectResolution('ApplyExternal')}
						class="accent-blue-600"
					/>
					Apply external changes
				</label>
				<label class="flex items-center gap-2 text-sm">
					<input
						type="radio"
						name="conflict-resolution"
						value="KeepEditing"
						data-testid="screen-4-resolution-keep-editing"
						checked={store.resolution === 'KeepEditing'}
						onchange={() => store.selectResolution('KeepEditing')}
						class="accent-blue-600"
					/>
					Keep my edits
				</label>
			</fieldset>

			<footer class="flex justify-end gap-2 pt-2">
				<button
					type="button"
					data-testid="screen-4-cancel"
					class="rounded-md border border-neutral-200 bg-white px-3 py-1 text-xs hover:bg-neutral-100 dark:border-neutral-700 dark:bg-neutral-800 dark:hover:bg-neutral-700"
					onclick={handleCancel}
				>
					Cancel
				</button>
				<button
					type="button"
					data-testid="screen-4-confirm"
					class="rounded-md bg-blue-600 px-3 py-1 text-xs font-medium text-white hover:bg-blue-700"
					onclick={handleApply}
				>
					Apply
				</button>
			</footer>
		</form>
	</dialog>
{/if}
