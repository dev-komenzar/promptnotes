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
	let tagInputDraft = $state('');
	let tagError = $state<string | null>(null);

	async function runSubmit(): Promise<boolean> {
		// タグ入力欄に未確定のテキストがあれば先にコミットする
		const pendingTag = tagInputDraft.trim();
		if (pendingTag !== '') {
			tagError = null;
			const result = store.addTag(pendingTag);
			if (result.outcome === 'added') {
				tagInputDraft = '';
			}
		}

		const bodySnapshot = store.body;
		const tagsSnapshot = [...store.tags];
		const outcome = await store.submit();
		if (outcome.outcome === 'created') {
			feed.prependNote({
				id: outcome.id,
				body: bodySnapshot,
				tags: tagsSnapshot,
				created_at: outcome.created_at,
				updated_at: outcome.created_at
			});
		}
		// 成否にかかわらずタグ入力欄をリセット
		tagInputDraft = '';
		tagError = null;
		return outcome.outcome === 'created';
	}

	function handleTagInputKey(event: KeyboardEvent): void {
		if (event.key === 'Enter') {
			event.preventDefault();
			const raw = tagInputDraft.trim();
			if (raw === '') return;
			tagError = null;
			const result = store.addTag(raw);
			if (result.outcome === 'invalid') {
				tagError = result.reason;
			} else {
				tagInputDraft = '';
			}
		}
	}

	function handleTagRemove(tag: string): void {
		store.removeTag(tag);
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
	aria-label="New note"
	class="sticky top-0 z-10 shrink-0 border-b border-neutral-200 bg-white px-3 py-2 dark:border-neutral-800 dark:bg-neutral-950"
>
	<div class="flex flex-col gap-1.5">
		<div class="flex min-h-[1.25rem] flex-wrap items-center gap-1 px-1 text-xs text-neutral-500">
			{#each store.tags as tag (tag)}
				<span
					class="inline-flex items-center gap-0.5 rounded-full bg-neutral-100 px-1.5 py-0.5 text-[10px] text-neutral-600 dark:bg-neutral-800 dark:text-neutral-300"
				>
					<span data-testid="screen-1-draft-tag-chip">#{tag}</span>
					<button
						type="button"
						data-testid="screen-1-draft-tag-remove"
						class="text-neutral-400 hover:text-red-500"
						aria-label={`Remove tag ${tag}`}
						onclick={() => handleTagRemove(tag)}>×</button
					>
				</span>
			{/each}
			<input
				type="text"
				data-testid="screen-1-draft-tag-input"
				class="min-w-[6rem] rounded border border-neutral-200 bg-white px-1 py-0.5 text-[10px] dark:border-neutral-700 dark:bg-neutral-900"
				placeholder="+ tag"
				aria-label="New tag"
				bind:value={tagInputDraft}
				onkeydown={handleTagInputKey}
			/>
			{#if tagError}
				<p class="text-[10px] text-red-500" data-testid="screen-1-draft-tag-error">{tagError}</p>
			{/if}
		</div>
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
				aria-label="Add new note (Cmd+Enter)"
				disabled={store.submitting}
				onclick={handleButtonClick}
			>
				+ Add
			</button>
		</div>
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
