<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { EditorView } from '@codemirror/view';
	import { createEditorState } from '../codemirror/setup';
	import { draftStore } from '../stores/draft.svelte';
	import { feedStore } from '../stores/feed.svelte';

	type Props = {
		store?: typeof draftStore;
		feed?: typeof feedStore;
	};

	let { store = draftStore, feed = feedStore }: Props = $props();

	let host: HTMLDivElement | undefined = $state();
	let view: EditorView | undefined;
	let suppressNextChange = false;

	async function runSubmit(): Promise<boolean> {
		const snapshot = store.body;
		const outcome = await store.submit();
		if (outcome.outcome === 'created') {
			feed.prependNote({
				id: outcome.id,
				body: snapshot,
				tags: [],
				created_at: outcome.created_at,
				updated_at: outcome.created_at
			});
		}
		return outcome.outcome === 'created';
	}

	function submitFromKeymap(): boolean {
		void runSubmit();
		return true;
	}

	function handleButtonClick(): void {
		void runSubmit();
	}

	onMount(() => {
		if (!host) return;
		const state = createEditorState({
			doc: store.body,
			onSubmit: submitFromKeymap,
			onChange: (next) => {
				if (suppressNextChange) {
					suppressNextChange = false;
					return;
				}
				store.setBody(next);
			}
		});
		view = new EditorView({ state, parent: host });
	});

	$effect(() => {
		if (!view) return;
		const next = store.body;
		const current = view.state.doc.toString();
		if (next === current) return;
		suppressNextChange = true;
		view.dispatch({
			changes: { from: 0, to: current.length, insert: next }
		});
	});

	onDestroy(() => {
		view?.destroy();
	});
</script>

<section
	data-testid="region-draft"
	aria-label="新規 Note"
	class="sticky top-0 z-10 shrink-0 border-b border-neutral-200 bg-white px-3 py-2 dark:border-neutral-800 dark:bg-neutral-950"
>
	<div class="flex items-start gap-2">
		<div
			bind:this={host}
			data-testid="screen-1-draft-body"
			class="min-h-[3rem] flex-1 rounded border border-neutral-200 bg-white px-2 py-1 text-sm focus-within:border-blue-500 dark:border-neutral-700 dark:bg-neutral-900"
		></div>
		<button
			type="button"
			data-testid="screen-1-draft-submit"
			class="shrink-0 self-stretch rounded-md bg-blue-600 px-3 text-sm font-medium text-white hover:bg-blue-500 disabled:cursor-not-allowed disabled:opacity-50"
			aria-label="新規 Note を追加 (Cmd+Enter)"
			disabled={store.submitting}
			onclick={handleButtonClick}
		>
			＋追加
		</button>
	</div>
</section>

<style>
	div[data-testid='screen-1-draft-body'] :global(.cm-editor) {
		outline: none;
	}
	div[data-testid='screen-1-draft-body'] :global(.cm-editor .cm-scroller) {
		font-family:
			ui-monospace, SFMono-Regular, 'SF Mono', Menlo, Consolas, 'Liberation Mono', monospace;
	}
</style>
