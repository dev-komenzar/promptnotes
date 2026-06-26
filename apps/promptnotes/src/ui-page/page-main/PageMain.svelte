<script lang="ts">
	import { loadSettings } from '$lib/user-preferences/slices/load-settings';
	import DraftRegion from './regions/DraftRegion.svelte';
	import FeedRegion from './regions/FeedRegion.svelte';
	import ToastRegion from './regions/ToastRegion.svelte';
	import ToolbarRegion from './regions/ToolbarRegion.svelte';
	import { feedStore } from './stores/feed.svelte';
	import { toastStore } from './stores/toasts.svelte';

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

	$effect(() => {
		toastStore.setOnRestored((note) => feedStore.prependNote(note));
		return () => toastStore.setOnRestored(undefined);
	});

	function handleOpenSettings() {
		// widget-settings-modal は別 epic。現時点では mount trigger 用 flag を
		// 立てるだけの placeholder（実体 widget は未実装）。
		settingsModalOpen = true;
		onOpenSettings?.();
	}

	function isEditableTarget(target: EventTarget | null): boolean {
		if (!(target instanceof Element)) return false;
		if (target.closest('.cm-editor')) return true;
		const tag = target.tagName;
		if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return true;
		if (target instanceof HTMLElement && target.isContentEditable) return true;
		return false;
	}

	function handleWindowKeydown(event: KeyboardEvent): void {
		// Cmd+Z (macOS) / Ctrl+Z (others) — グローバル Undo は editor 上では譲る。
		if ((event.metaKey || event.ctrlKey) && !event.shiftKey && !event.altKey && event.key === 'z') {
			if (isEditableTarget(event.target)) return;
			if (toastStore.entries.length === 0) return;
			event.preventDefault();
			void toastStore.undoLatest();
		}
	}
</script>

<svelte:window onkeydown={handleWindowKeydown} />

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
