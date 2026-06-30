<script lang="ts">
	import { onDestroy, onMount, untrack } from 'svelte';
	import { fade } from 'svelte/transition';
	import { EditorView } from '@codemirror/view';
	import { assignTag } from '$lib/note-capture/slices/assign-tag';
	import { autoSaveNote } from '$lib/note-capture/slices/auto-save-note';
	import { copyNoteBody } from '$lib/note-capture/slices/copy-note-body';
	import { deleteNote } from '$lib/note-capture/slices/delete-note';
	import { flushNote } from '$lib/note-capture/slices/flush-note';
	import { removeTag } from '$lib/note-capture/slices/remove-tag';
	import type { FlushTrigger } from '$lib/note-capture/slices/flush-note';
	import { createEditorState } from '../codemirror/setup';
	import type { FocusStore, BlockState } from '../stores/focus.svelte';
	import type { FeedStore, NoteSummary } from '../stores/feed.svelte';
	import { pendingFlushRegistry, type PendingFlushRegistry } from '../stores/pending-flush.svelte';
	import { toastStore } from '../stores/toasts.svelte';

	type Props = {
		note: NoteSummary;
		focus: FocusStore;
		feed: FeedStore;
		onTagFilter: (tag: string) => void;
		autoSaveDebounceMs?: number;
		assignTagFn?: typeof assignTag;
		removeTagFn?: typeof removeTag;
		autoSaveFn?: typeof autoSaveNote;
		flushFn?: typeof flushNote;
		deleteFn?: typeof deleteNote;
		copyFn?: typeof copyNoteBody;
		toasts?: typeof toastStore;
		pendingFlush?: PendingFlushRegistry;
	};

	let {
		note,
		focus,
		feed,
		onTagFilter,
		autoSaveDebounceMs = 600,
		assignTagFn = assignTag,
		removeTagFn = removeTag,
		autoSaveFn = autoSaveNote,
		flushFn = flushNote,
		deleteFn = deleteNote,
		copyFn = copyNoteBody,
		toasts = toastStore,
		pendingFlush = pendingFlushRegistry
	}: Props = $props();

	let host: HTMLDivElement | undefined = $state();
	let blockEl: HTMLElement | undefined = $state();
	let view: EditorView | undefined;
	let suppressNextChange = false;
	let debounceTimer: ReturnType<typeof setTimeout> | null = null;
	let pendingBody: string | null = null;
	let tagInputDraft = $state('');
	let tagError = $state<string | null>(null);
	let copied = $state(false);
	let copyTimer: ReturnType<typeof setTimeout> | null = null;
	let pendingClickPos: number | null = $state(null);

	const blockState = $derived<BlockState>(focus.stateOf(note.id));

	function cancelDebounce(): void {
		if (debounceTimer !== null) {
			clearTimeout(debounceTimer);
			debounceTimer = null;
		}
	}

	function scheduleAutoSave(body: string): void {
		pendingBody = body;
		// ori-73q / spec.md#impl-quit-orchestration: pendingBody を抱えた瞬間に
		// quit registry へ登録。timer fire / flush 完了で unregister する。
		pendingFlush.register(note.id, runFlush);
		cancelDebounce();
		debounceTimer = setTimeout(() => {
			debounceTimer = null;
			const target = pendingBody;
			if (target === null) return;
			pendingBody = null;
			pendingFlush.unregister(note.id);
			void autoSaveFn(note.id, target)
				.then((outcome) => {
					if (outcome.outcome === 'saved') {
						feed.applyAutoSave(outcome.id, outcome.updated_at);
					}
				})
				.catch(() => {
					// MVP: silent on failure; toast surface handled in page-main-toast sub-task
				});
		}, autoSaveDebounceMs);
	}

	async function runFlush(trigger: FlushTrigger): Promise<void> {
		cancelDebounce();
		const target = pendingBody;
		pendingBody = null;
		pendingFlush.unregister(note.id);
		if (target === null) return;
		try {
			const outcome = await flushFn(note.id, target, trigger);
			if (outcome.outcome === 'flushed') {
				feed.applyAutoSave(outcome.id, outcome.updated_at);
			}
		} catch {
			// MVP silent
		}
	}

	onMount(() => {
		if (!host) return;
		const initState = createEditorState({
			doc: note.body,
			readOnly: blockState !== 'EDITING',
			onSubmit: () => false,
			onChange: (next) => {
				if (suppressNextChange) {
					suppressNextChange = false;
					return;
				}
				if (blockState !== 'EDITING') return;
				feed.applyBodyEdit(note.id, next);
				scheduleAutoSave(next);
			}
		});
		view = new EditorView({ state: initState, parent: host });
	});

	// Sync external body changes (e.g. note replaced) back to CM doc.
	$effect(() => {
		if (!view) return;
		const next = note.body;
		const current = view.state.doc.toString();
		if (next === current) return;
		suppressNextChange = true;
		view.dispatch({ changes: { from: 0, to: current.length, insert: next } });
	});

	// Rebuild editor state when readOnly should flip.
	$effect(() => {
		const editing = blockState === 'EDITING';
		if (!view) return;
		untrack(() => {
			if (!view) return;
			const next = createEditorState({
				doc: view.state.doc.toString(),
				readOnly: !editing,
				onSubmit: () => false,
				onChange: (text) => {
					if (suppressNextChange) {
						suppressNextChange = false;
						return;
					}
					if (focus.stateOf(note.id) !== 'EDITING') return;
					feed.applyBodyEdit(note.id, text);
					scheduleAutoSave(text);
				}
			});
			view.setState(next);
			if (editing) {
				view.focus();
				// I-PM10a: restore cursor to click position instead of position 0
				if (pendingClickPos !== null) {
					view.dispatch({
						selection: { anchor: pendingClickPos },
						scrollIntoView: false
					});
					pendingClickPos = null;
				}
			}
		});
	});

	// When transitioning out of EDITING (any cause), flush pending body.
	let wasEditing = false;
	$effect(() => {
		const editing = blockState === 'EDITING';
		if (wasEditing && !editing) {
			void runFlush('block_blur');
		}
		wasEditing = editing;
	});

	onDestroy(() => {
		cancelDebounce();
		pendingFlush.unregister(note.id);
		view?.destroy();
	});

	function handleBlockClick(event: MouseEvent): void {
		const target = event.target as HTMLElement;
		// Don't promote to EDITING when clicking on hover-actions or tag controls.
		if (target.closest('[data-block-no-edit]')) return;
		// Capture click position in CM document coords before state transition.
		// I-PM10a: cursor must land where the user clicked, not at position 0.
		if (view && blockState !== 'EDITING') {
			const pos = view.posAtCoords({ x: event.clientX, y: event.clientY });
			if (pos !== null) {
				pendingClickPos = pos;
			}
		}
		focus.edit(note.id);
	}

	function handleKeyDown(event: KeyboardEvent): void {
		if (blockState === 'EDITING') {
			if (event.key === 'Escape') {
				event.preventDefault();
				focus.escape();
			}
			return;
		}
		// FOCUSED / IDLE on Block element
		if (event.key === 'Enter') {
			event.preventDefault();
			focus.enter();
		} else if (event.key === 'Escape') {
			event.preventDefault();
			focus.escape();
		}
	}

	function handleTagChipClick(tag: string): void {
		onTagFilter(tag);
	}

	async function handleTagRemove(tag: string): Promise<void> {
		try {
			const outcome = await removeTagFn(note.id, tag);
			if (outcome.outcome === 'removed') {
				feed.applyRemoveTag(outcome.id, outcome.tags, outcome.updated_at);
			}
		} catch {
			// MVP silent
		}
	}

	async function submitTagInput(): Promise<void> {
		const raw = tagInputDraft.trim();
		if (raw === '') return;
		tagError = null;
		try {
			const outcome = await assignTagFn(note.id, raw);
			if (outcome.outcome === 'assigned') {
				feed.applyAssignTag(outcome.id, outcome.tags, outcome.updated_at);
				tagInputDraft = '';
			} else {
				tagInputDraft = '';
			}
		} catch (err) {
			const e = err as { kind?: string; reason?: string };
			if (e?.kind === 'invalid_tag') {
				tagError = 'Tag contains invalid characters (comma, brackets, whitespace).';
			}
		}
	}

	function handleTagInputKey(event: KeyboardEvent): void {
		if (event.key === 'Enter') {
			event.preventDefault();
			void submitTagInput();
		}
	}

	async function handleDelete(): Promise<void> {
		// Capture a snapshot now so toast restoration can recover original created_at.
		const snapshot: NoteSummary = {
			id: note.id,
			body: note.body,
			tags: [...note.tags],
			created_at: note.created_at,
			updated_at: note.updated_at
		};
		try {
			await deleteFn(note.id);
		} catch (err) {
			// Backend rejected the delete — keep the note visible and surface for debugging.
			console.error('[ori-6aa] delete_note failed:', err);
			return;
		}
		// ori-6aa: push toast before applyDelete. Mutating the feed first removes
		// this Block from the keyed {#each}, which can interrupt the subsequent
		// toasts.push (Undo) before reactivity propagates to ToastRegion.
		toasts.push(snapshot);
		feed.applyDelete(note.id);
	}

	async function handleCopy(): Promise<void> {
		try {
			await copyFn(note.id);
		} catch {
			// optimistic feedback: show ✅ regardless
			console.error('copy_note_body failed', note.id);
		}
		if (copyTimer !== null) {
			clearTimeout(copyTimer);
		}
		copied = true;
		copyTimer = setTimeout(() => {
			copied = false;
			copyTimer = null;
		}, 1500);
	}

	$effect(() => {
		if (blockState === 'FOCUSED' && blockEl && document.activeElement !== blockEl) {
			blockEl.focus({ preventScroll: false });
		}
	});
</script>

<!-- svelte-ignore a11y_no_redundant_roles -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<article
	bind:this={blockEl}
	role="article"
	tabindex="0"
	data-testid="screen-1-block"
	data-block-id={note.id}
	data-block-state={blockState}
	aria-label={`Note (created ${note.created_at})`}
	class={[
		'group relative flex flex-col gap-1 border-b border-neutral-200 px-3 py-2 text-sm focus:outline-none dark:border-neutral-800',
		blockState !== 'IDLE' ? 'bg-blue-50/60 dark:bg-blue-900/20' : 'bg-white dark:bg-neutral-950'
	]}
	onclick={handleBlockClick}
	onkeydown={handleKeyDown}
>
	<header
		class="flex min-h-[1.25rem] flex-wrap items-center gap-1.5 text-xs text-neutral-500 dark:text-neutral-400"
	>
		<div class="flex min-w-0 flex-1 flex-wrap items-center gap-1">
			{#each note.tags as tag (tag)}
				<span
					class="inline-flex items-center gap-0.5 rounded-full bg-neutral-100 px-1.5 py-0.5 text-[10px] text-neutral-600 dark:bg-neutral-800 dark:text-neutral-300"
				>
					<button
						type="button"
						data-block-no-edit
						data-testid="screen-1-block-tag-chip"
						class="hover:text-blue-600 dark:hover:text-blue-400"
						aria-label={`Filter by tag ${tag}`}
						onclick={(e) => {
							e.stopPropagation();
							handleTagChipClick(tag);
						}}
					>
						#{tag}
					</button>
					{#if blockState === 'EDITING'}
						<button
							type="button"
							data-block-no-edit
							data-testid="screen-1-block-tag-remove"
							class="text-neutral-400 hover:text-red-500"
							aria-label={`Remove tag ${tag}`}
							onclick={(e) => {
								e.stopPropagation();
								void handleTagRemove(tag);
							}}>×</button
						>
					{/if}
				</span>
			{/each}
			{#if blockState === 'EDITING'}
				<input
					type="text"
					data-block-no-edit
					data-testid="screen-1-block-tag-input"
					class="min-w-[6rem] rounded border border-neutral-200 bg-white px-1 py-0.5 text-[10px] dark:border-neutral-700 dark:bg-neutral-900"
					placeholder="+ tag"
					aria-label="New tag"
					bind:value={tagInputDraft}
					onkeydown={handleTagInputKey}
					onclick={(e) => e.stopPropagation()}
				/>
			{/if}
		</div>
		<time
			data-testid="screen-1-block-created-at"
			class="shrink-0 text-right text-[10px] tabular-nums text-neutral-400"
			datetime={note.created_at}
			title={`updated_at: ${note.updated_at}`}
		>
			{note.created_at}
		</time>
	</header>
	{#if tagError}
		<p class="text-[10px] text-red-500" data-testid="screen-1-block-tag-error">{tagError}</p>
	{/if}
	<div bind:this={host} data-testid="screen-1-block-body" class="w-full text-sm"></div>

	<div
		class="pointer-events-none absolute right-2 top-1 flex gap-1 opacity-0 transition-opacity group-hover:pointer-events-auto group-hover:opacity-100"
	>
		<button
			type="button"
			data-block-no-edit
			data-testid="screen-1-block-copy"
			class="rounded border border-neutral-200 bg-white px-1.5 py-0.5 text-[10px] hover:bg-neutral-100 dark:border-neutral-700 dark:bg-neutral-900 dark:hover:bg-neutral-800 grid place-items-center"
			aria-label="Copy body"
			onclick={(e) => {
				e.stopPropagation();
				void handleCopy();
			}}
		>
			{#key copied}
				{#if copied}
					<span in:fade={{ duration: 150 }} out:fade={{ duration: 150 }} style="grid-area: 1/1"
						>✅</span
					>
				{:else}
					<span in:fade={{ duration: 150 }} out:fade={{ duration: 150 }} style="grid-area: 1/1"
						>📋</span
					>
				{/if}
			{/key}
		</button>
		<button
			type="button"
			data-block-no-edit
			data-testid="screen-1-block-delete"
			class="rounded border border-red-200 bg-white px-1.5 py-0.5 text-[10px] text-red-600 hover:bg-red-50 dark:border-red-800 dark:bg-neutral-900 dark:hover:bg-red-900/20"
			aria-label="Delete note"
			onclick={(e) => {
				e.stopPropagation();
				void handleDelete();
			}}>🗑️</button
		>
	</div>
</article>

<style>
	article :global(.cm-editor) {
		outline: none;
		background: transparent;
	}
	article :global(.cm-editor .cm-scroller) {
		font-family:
			ui-monospace, SFMono-Regular, 'SF Mono', Menlo, Consolas, 'Liberation Mono', monospace;
	}
</style>
