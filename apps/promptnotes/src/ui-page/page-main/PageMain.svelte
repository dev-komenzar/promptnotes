<script lang="ts">
	import ToolbarRegion from './regions/ToolbarRegion.svelte';
	import DraftRegion from './regions/DraftRegion.svelte';
	import FeedRegion from './regions/FeedRegion.svelte';
	import ToastRegion from './regions/ToastRegion.svelte';
</script>

<div
	data-testid="page-main"
	class="flex h-screen min-h-0 w-screen flex-col overflow-hidden bg-white text-neutral-900 dark:bg-neutral-950 dark:text-neutral-100"
>
	<ToolbarRegion />
	<DraftRegion />
	<FeedRegion />
	<ToastRegion />
</div>
