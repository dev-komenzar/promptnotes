<script lang="ts">
	import { untrack } from 'svelte';
	import {
		createExternalDeleteNoticeStore,
		type ExternalDeleteNoticeStoreDeps
	} from './store.svelte';

	type Props = {
		/** store deps の上書き（テスト容易性 / page-main からの実配線） */
		deps?: ExternalDeleteNoticeStoreDeps;
	};

	let { deps }: Props = $props();

	const store = untrack(() => createExternalDeleteNoticeStore(deps));

	let dialogEl: HTMLDialogElement | null = $state(null);

	$effect(() => {
		store.start();
		return () => store.stop();
	});

	$effect(() => {
		if (store.state === 'notice') {
			dialogEl?.showModal();
		}
	});

	async function handleSaveAsNew() {
		await store.saveAsNew();
		dialogEl?.close();
	}

	async function handleDiscard() {
		await store.discard();
		dialogEl?.close();
	}

	function handleDialogCancel(event: Event) {
		// S20 は明示的な 2 択のみ。Esc で DOM だけ閉じて状態が取り残されるのを防ぐ。
		event.preventDefault();
	}
</script>

{#if store.payload}
	<dialog
		bind:this={dialogEl}
		data-testid="widget-external-delete-notice"
		aria-labelledby="widget-delete-notice-title"
		class="rounded-lg border border-neutral-200 bg-white p-0 text-neutral-900 shadow-xl backdrop:bg-transparent dark:border-neutral-800 dark:bg-neutral-900 dark:text-neutral-100"
		oncancel={handleDialogCancel}
	>
		<form method="dialog" class="flex w-[36rem] max-w-[90vw] flex-col gap-4 p-5">
			<h1
				id="widget-delete-notice-title"
				class="text-base font-semibold"
				data-testid="delete-notice-title"
			>
				{store.payload.note_title}
			</h1>

			<p class="text-sm text-neutral-600 dark:text-neutral-400" data-testid="delete-notice-message">
				編集中のノート「<span class="font-mono">{store.payload.note_title}</span
				>」が外部で削除されました。
				現在の内容で新規ファイルとして保存するか、破棄するかを選んでください。
			</p>

			<textarea
				data-testid="delete-notice-body-local"
				readonly
				disabled
				value={store.payload.note_body}
				rows={6}
				class="w-full resize-none rounded-md border border-neutral-200 bg-neutral-50 p-2 font-mono text-xs text-neutral-700 dark:border-neutral-700 dark:bg-neutral-800 dark:text-neutral-300"
			></textarea>

			<footer class="flex justify-end gap-2 pt-2">
				<button
					type="button"
					data-testid="delete-notice-discard"
					class="rounded-md border border-neutral-200 bg-white px-3 py-1 text-xs hover:bg-neutral-100 dark:border-neutral-700 dark:bg-neutral-800 dark:hover:bg-neutral-700"
					onclick={handleDiscard}
				>
					破棄
				</button>
				<button
					type="button"
					data-testid="delete-notice-save-as-new"
					class="rounded-md bg-blue-600 px-3 py-1 text-xs font-medium text-white hover:bg-blue-700"
					onclick={handleSaveAsNew}
				>
					新規ファイルとして保存
				</button>
			</footer>
		</form>
	</dialog>
{/if}
