<script lang="ts">
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import { listNotes } from '$lib/note-feed/slices/list-feed';
	import { loadSettings, type Settings } from '$lib/user-preferences/slices/load-settings';
	import type { SettingsDto } from '$lib/user-preferences/slices/update-settings';
	import WidgetSettingsModal from '../../ui-widget/settings-modal/WidgetSettingsModal.svelte';
	import WidgetUpdateToast from '../../ui-widget/update-toast/WidgetUpdateToast.svelte';
	import DraftRegion from './regions/DraftRegion.svelte';
	import FeedRegion from './regions/FeedRegion.svelte';
	import ToastRegion from './regions/ToastRegion.svelte';
	import ToolbarRegion from './regions/ToolbarRegion.svelte';
	import { feedStore } from './stores/feed.svelte';
	import {
		pendingFlushRegistry,
		type PendingFlushRegistry
	} from './stores/pending-flush.svelte';
	import { toastStore } from './stores/toasts.svelte';

	type CloseRequestedEvent = { preventDefault: () => void };
	type QuitWindow = {
		onCloseRequested: (cb: (event: CloseRequestedEvent) => void | Promise<void>) => Promise<() => void>;
		destroy: () => Promise<void>;
	};

	type Props = {
		onOpenSettings?: () => void;
		loadSettingsFn?: typeof loadSettings;
		listNotesFn?: typeof listNotes;
		pendingFlush?: PendingFlushRegistry;
		quitWindow?: QuitWindow | null;
	};

	let {
		onOpenSettings,
		loadSettingsFn = loadSettings,
		listNotesFn = listNotes,
		pendingFlush = pendingFlushRegistry,
		quitWindow
	}: Props = $props();

	const DEFAULT_SETTINGS: Settings = {
		storage_dir: '',
		theme: 'System',
		sort_preference: { field: 'created_at', direction: 'desc' }
	};

	let settingsModalOpen = $state(false);
	let currentSettings = $state<Settings>({ ...DEFAULT_SETTINGS });

	$effect(() => {
		// I-PM3 partial: load-settings → list-feed on mount.
		// Silent fallback per aggregates.md#settings-loading (no warning region).
		// list-feed slice (workflows/list-feed.md): storage_dir/*.md を hydrate して
		// feedStore.notes を初期化する。Rust 側で sort も適用済 (sort_preference 復元) なので
		// hydrateSort と順序は前後しても visible 結果は変わらない。
		void loadSettingsFn()
			.then((settings) => {
				currentSettings = settings;
				feedStore.hydrateSort(settings.sort_preference);
			})
			.catch(() => {
				// silent fallback to defaults
			});
		void listNotesFn()
			.then((feed) => {
				feedStore.hydrateNotes(feed.notes);
			})
			.catch(() => {
				// silent fallback: feedStore.notes stays empty
			});
	});

	$effect(() => {
		toastStore.setOnRestored((note) => feedStore.prependNote(note));
		return () => toastStore.setOnRestored(undefined);
	});

	// ori-73q / spec.md#impl-quit-orchestration: S13 連続 Flush の orchestration。
	// Tauri の CloseRequested を frontend で intercept → preventDefault →
	// 全 pending Note を順次 flush → window.destroy() で実際に閉じる。
	// quitWindow が明示注入されたらそれを使う (test injection)。null 注入なら hook を skip。
	// 未指定 (本番) は getCurrentWindow() を resolve する (browser 環境では失敗 → skip)。
	$effect(() => {
		if (quitWindow === null) return;
		let unlisten: (() => void) | undefined;
		let disposed = false;
		const resolveWindow = (): QuitWindow | null => {
			if (quitWindow) return quitWindow;
			try {
				return getCurrentWindow() as unknown as QuitWindow;
			} catch {
				return null;
			}
		};
		const win = resolveWindow();
		if (!win) return;
		void win
			.onCloseRequested(async (event) => {
				event.preventDefault();
				try {
					await pendingFlush.flushAll('app_quit');
				} catch (err) {
					// flushAll は個別失敗を swallow する実装だが、念のため最終 catch
					console.error('[quit] flushAll failed', err);
				}
				try {
					await win.destroy();
				} catch (err) {
					// destroy() が permission denied 等で失敗するとアプリが終了しない。
					// silent rejection だと UX 上ユーザーが原因を追えないので必ず log する。
					console.error('[quit] window.destroy() failed', err);
				}
			})
			.then((u) => {
				if (disposed) u();
				else unlisten = u;
			})
			.catch(() => {
				// silent — non-Tauri host (test / browser) は CloseRequested 不要
			});
		return () => {
			disposed = true;
			unlisten?.();
		};
	});

	function handleOpenSettings() {
		settingsModalOpen = true;
		onOpenSettings?.();
	}

	function handleSettingsModalClose() {
		settingsModalOpen = false;
	}

	function handleSettingsSaved(next: SettingsDto) {
		// Settings 変更後の即時反映: in-memory state を更新し、新 storage_dir で Feed を再 hydrate する。
		// Rust 側 list_notes は呼び出し毎に settings.json を読み直すため、frontend が re-invoke するだけで足りる。
		currentSettings = { ...next };
		feedStore.hydrateSort(next.sort_preference);
		void listNotesFn()
			.then((feed) => {
				feedStore.hydrateNotes(feed.notes);
			})
			.catch(() => {
				// silent fallback: feed stays as-is
			});
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
	<WidgetSettingsModal
		initial={settingsForModal()}
		onClose={handleSettingsModalClose}
		onSaved={handleSettingsSaved}
	/>
{/if}

<WidgetUpdateToast />
