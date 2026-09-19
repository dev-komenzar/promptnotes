<script lang="ts">
		import { getCurrentWindow } from '@tauri-apps/api/window';
		import { listen } from '@tauri-apps/api/event';
		import { listNotes } from '$lib/note-feed/slices/list-feed';
		import { loadSettings, type Settings } from '$lib/user-preferences/slices/load-settings';
		import type { SettingsDto } from '$lib/user-preferences/slices/update-settings';
		import WidgetExternalChangeConflict from '../../ui-widget/external-change-conflict/WidgetExternalChangeConflict.svelte';
		import WidgetSettingsModal from '../../ui-widget/settings-modal/WidgetSettingsModal.svelte';
		import WidgetUpdateToast from '../../ui-widget/update-toast/WidgetUpdateToast.svelte';
		import DraftRegion from './regions/DraftRegion.svelte';
		import FeedRegion from './regions/FeedRegion.svelte';
		import ToastRegion from './regions/ToastRegion.svelte';
		import ToolbarRegion from './regions/ToolbarRegion.svelte';
		import { editingNote, type EditingNoteState } from './stores/editing-note.svelte';
		import { feedStore } from './stores/feed.svelte';
		import { focusStore } from './stores/focus.svelte';
		import { pendingFlushRegistry, type PendingFlushRegistry } from './stores/pending-flush.svelte';
		import { createSortPreferenceSubscriber } from './stores/sort-preference-subscriber.svelte';
		import { createThemeSubscriber } from './stores/theme-subscriber.svelte';
		import { toastStore } from './stores/toasts.svelte';

	type CloseRequestedEvent = { preventDefault: () => void };
	type QuitWindow = {
		onCloseRequested: (
			cb: (event: CloseRequestedEvent) => void | Promise<void>
		) => Promise<() => void>;
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
		let restartPromptOpen = $state(false);
		let currentSettings = $state<Settings>({ ...DEFAULT_SETTINGS });
		let draftRegion: ReturnType<typeof DraftRegion> | undefined;

		const themeSubscriber = createThemeSubscriber({
		onThemeChanged: (theme) => {
			// theme_changed event で currentSettings.theme を更新 (SSoT)。
			// 下の $effect が currentSettings.theme に reactive に反応して setTheme → DOM 反映する。
			currentSettings = { ...currentSettings, theme };
		}
	});

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

	$effect(() => {
		const subscriber = createSortPreferenceSubscriber();
		void subscriber.start();
		return () => subscriber.stop();
	});

		$effect(() => {
			void themeSubscriber.start();
			return () => themeSubscriber.stop();
		});

		$effect(() => {
			// Start file watcher for external change detection (Syncthing support).
			let cancelled = false;
			(async () => {
				try {
					const { invoke } = await import('@tauri-apps/api/core');
					if (cancelled) return;
					await invoke('start_file_watcher').catch(() => {});
				} catch {
					// silent — non-Tauri host (e.g. vitest jsdom)
				}
			})();
			return () => { cancelled = true; };
		});

		$effect(() => {
			// Subscribe to external file change events emitted by the watcher.
			// When Syncthing (or any external program) creates/modifies/deletes .md files,
			// the Rust watcher publishes domain events → subscriber updates NoteFeed
			// and emits 'notes-changed' → frontend re-fetches the feed.
			let unlisten: (() => void) | undefined;
			(async () => {
				try {
					unlisten = await listen('notes-changed', async () => {
						try {
							const feed = await listNotesFn();
							feedStore.hydrateNotes(feed.notes);
						} catch {
							// silent — re-hydration failure preserves current feed
						}
					});
				} catch {
					// silent — non-Tauri host
				}
			})();
			return () => {
				unlisten?.();
			};
		});

	$effect(() => {
		// S11 / I-S4: StorageDirChanged subscriber。storage_dir 変更時は再起動を促すモーダルを表示し、
		// Feed は旧ディレクトリのまま維持する (domain-events.md#storage-dir-changed-subscribers)。
		let unlisten: (() => void) | undefined;
		let disposed = false;
		(async () => {
			try {
				const u = await listen('settings:storage_dir_changed', () => {
					restartPromptOpen = true;
				});
				if (disposed) u();
				else unlisten = u;
			} catch {
				// silent — non-Tauri host (e.g. vitest jsdom)
			}
		})();
		return () => {
			disposed = true;
			unlisten?.();
		};
	});

	$effect(() => {
		// currentSettings.theme が変わったら DOM に反映 (I-PM16/17/18)。
		// load-settings 後 / theme_changed event 後 / settings save 後 の全 case を cover。
		themeSubscriber.setTheme(currentSettings.theme);
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
		// S11 / I-S4: storage_dir 変更は即時マイグレーションしない。Feed を新ディレクトリで再 hydrate すると
		// 旧ディレクトリの Note が消えてしまうため、storage_dir 非変更時のみ再 hydrate する。
		// 再起動要求は settings:storage_dir_changed subscriber が表示する。
		const storageDirChanged = next.storage_dir !== currentSettings.storage_dir;
		currentSettings = { ...next };
		if (!storageDirChanged) {
			feedStore.hydrateSort(next.sort_preference);
			void listNotesFn()
				.then((feed) => {
					feedStore.hydrateNotes(feed.notes);
				})
				.catch(() => {
					// silent fallback: feed stays as-is
				});
		}
	}

	function handleRestartNow() {
		restartPromptOpen = false;
		if (typeof window !== 'undefined') window.location.reload();
	}

	function handleRestartLater() {
		restartPromptOpen = false;
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

		// Cmd+N (macOS) / Ctrl+N (others) — Draft エディタにフォーカスを移動する。
		// Feed block が EDITING 中でも常に発動する (isEditableTarget ガードは適用しない)。
		if ((event.metaKey || event.ctrlKey) && !event.shiftKey && !event.altKey && event.key.toLowerCase() === 'n') {
			if (event.isComposing) return;
			event.preventDefault();
			focusStore.clear();
			draftRegion?.focusDraft();
		}
	}
</script>

<svelte:window onkeydown={handleWindowKeydown} />

<div
	data-testid="page-main"
	data-settings-modal-open={settingsModalOpen}
	data-restart-prompt-open={restartPromptOpen}
	class="flex h-screen min-h-0 w-screen flex-col overflow-hidden bg-white text-neutral-900 dark:bg-neutral-950 dark:text-neutral-100"
>
	<ToolbarRegion onOpenSettings={handleOpenSettings} />
	<DraftRegion bind:this={draftRegion} />
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

{#if restartPromptOpen}
	<div
		role="alertdialog"
		aria-modal="true"
		aria-labelledby="restart-prompt-title"
		data-testid="restart-prompt"
		class="fixed inset-0 z-50 flex items-center justify-center"
	>
		<div
			class="w-[24rem] max-w-[90vw] rounded-lg border border-neutral-200 bg-white p-5 text-neutral-900 shadow-xl dark:border-neutral-800 dark:bg-neutral-900 dark:text-neutral-100"
		>
			<h2 id="restart-prompt-title" class="text-base font-semibold">Restart required</h2>
			<p class="mt-2 text-sm text-neutral-600 dark:text-neutral-300">
				The storage directory changed. Existing notes stay visible until you restart.
			</p>
			<div class="mt-4 flex justify-end gap-2">
				<button
					type="button"
					data-testid="restart-prompt-later"
					class="rounded-md border border-neutral-200 bg-white px-3 py-1 text-xs hover:bg-neutral-100 dark:border-neutral-700 dark:bg-neutral-800 dark:hover:bg-neutral-700"
					onclick={handleRestartLater}
				>
					Later
				</button>
				<button
					type="button"
					data-testid="restart-prompt-restart"
					class="rounded-md bg-blue-600 px-3 py-1 text-xs font-medium text-white hover:bg-blue-700"
					onclick={handleRestartNow}
				>
					Restart now
				</button>
			</div>
		</div>
	</div>
{/if}

<WidgetUpdateToast />

<WidgetExternalChangeConflict
	localBody=""
	onClose={() => {}}
/>
