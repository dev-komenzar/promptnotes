<script lang="ts">
	import { loadSettings, type Settings } from '$lib/user-preferences/slices/load-settings';
	import type { SettingsDto } from '$lib/user-preferences/slices/update-settings';
	import WidgetSettingsModal from '../../ui-widget/settings-modal/WidgetSettingsModal.svelte';
	import WidgetUpdateToast from '../../ui-widget/update-toast/WidgetUpdateToast.svelte';
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

	const DEFAULT_SETTINGS: Settings = {
		storage_dir: '',
		theme: 'System',
		sort_preference: { field: 'created_at', direction: 'desc' }
	};

	let settingsModalOpen = $state(false);
	let currentSettings = $state<Settings>({ ...DEFAULT_SETTINGS });

	$effect(() => {
		// I-PM3 partial: load-settings on mount, hydrate toolbar sort initial value.
		// Silent fallback per aggregates.md#settings-loading (no warning region).
		void loadSettingsFn()
			.then((settings) => {
				currentSettings = settings;
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
		settingsModalOpen = true;
		onOpenSettings?.();
	}

	function handleSettingsModalClose() {
		settingsModalOpen = false;
	}

	function settingsForModal(): SettingsDto {
		return { ...currentSettings };
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

{#if settingsModalOpen}
	<WidgetSettingsModal initial={settingsForModal()} onClose={handleSettingsModalClose} />
{/if}

<WidgetUpdateToast />
