<script lang="ts">
	import Block from '../components/Block.svelte';
	import { feedStore } from '../stores/feed.svelte';
	import { focusStore } from '../stores/focus.svelte';

	type Props = {
		feed?: typeof feedStore;
		focus?: typeof focusStore;
	};

	let { feed = feedStore, focus = focusStore }: Props = $props();

	function handleTagFilter(tag: string): void {
		void feed.setTag(tag);
	}

	function handleKeyDown(event: KeyboardEvent): void {
		// Container-level fallback for ArrowUp/ArrowDown when no Block has focus.
		if (focus.activeState === 'EDITING') return;
		if (event.key === 'ArrowDown') {
			event.preventDefault();
			focus.navigate('next', visibleIds);
		} else if (event.key === 'ArrowUp') {
			event.preventDefault();
			focus.navigate('prev', visibleIds);
		}
	}

	const visibleNotes = $derived(feed.visibleNotes);
	const visibleIds = $derived(visibleNotes.map((n) => n.id));
</script>

<svelte:window
	onblur={() => {
		if (focus.activeState === 'EDITING') focus.escape();
	}}
/>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<main
	data-testid="region-feed"
	aria-label="Note feed"
	class="flex flex-1 flex-col overflow-y-auto text-sm text-neutral-700 dark:text-neutral-200"
	onkeydown={handleKeyDown}
	role="feed"
	tabindex="-1"
>
	{#if visibleNotes.length === 0}
		<p
			data-testid="screen-1-feed-empty"
			class="px-3 py-6 text-center text-neutral-400 dark:text-neutral-500"
		>
			No notes match
		</p>
	{:else}
		{#each visibleNotes as note (note.id)}
			<Block {note} {focus} {feed} onTagFilter={handleTagFilter} />
		{/each}
	{/if}
</main>
