<script lang="ts">
	import { loadSettings } from '$lib/user-preferences/slices/load-settings';
	import DraftRegion from './regions/DraftRegion.svelte';
	import FeedRegion from './regions/FeedRegion.svelte';
	import ToastRegion from './regions/ToastRegion.svelte';
	import ToolbarRegion from './regions/ToolbarRegion.svelte';
	import { feedStore } from './stores/feed.svelte';

	type Props = {
		onOpenSettings?: () => void;
		loadSettingsFn?: typeof loadSettings;
	};

	let { onOpenSettings, loadSettingsFn = loadSettings }: Props = $props();

	let settingsModalOpen = $state(false);

	$effect(() => {
		// I-PM3 partial: load-settings on mount, hydrate toolbar sort initial value.
		// Silent fallback per aggregates.md#settings-loading (no warning region).
		void loadSettingsFn()
			.then((settings) => {
				feedStore.hydrateSort(settings.sort_preference);
			})
			.catch(() => {
				// silent fallback to defaults
			});
	});

	function handleOpenSettings() {
		// widget-settings-modal は別 epic。現時点では mount trigger 用 flag を
		// 立てるだけの placeholder（実体 widget は未実装）。
		settingsModalOpen = true;
		onOpenSettings?.();
	}
</script>

<div
	data-testid="page-main"
	data-settings-modal-open={settingsModalOpen}
	class="flex h-screen min-h-0 w-screen flex-col overflow-hidden bg-white text-neutral-900 dark:bg-neutral-950 dark:text-neutral-100"
>
	<ToolbarRegion onOpenSettings={handleOpenSettings} />
	<DraftRegion />
	<FeedRegion />
	<ToastRegion />
</div>
