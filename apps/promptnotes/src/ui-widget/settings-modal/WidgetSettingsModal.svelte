<script lang="ts">
	import { untrack } from 'svelte';
	import { open as openDialog } from '@tauri-apps/plugin-dialog';
	import type {
		SettingsDto,
		Theme,
		UpdateSettingsError
	} from '$lib/user-preferences/slices/update-settings';
	import { createSettingsModalStore } from './store.svelte';
	import { createSettingsUpdateStore, type SettingsUpdateStoreDeps } from './update-store.svelte';

	type Props = {
		initial: SettingsDto;
		onClose: () => void;
		onSaved?: (settings: SettingsDto) => void;
		updateSettingsFn?: Parameters<typeof createSettingsModalStore>[1] extends infer D
			? D extends { updateSettingsFn?: infer F }
				? F
				: never
			: never;
		getAppVersionFn?: Parameters<typeof createSettingsModalStore>[1] extends infer D
			? D extends { getAppVersionFn?: infer F }
				? F
				: never
			: never;
		getDefaultStorageDirFn?: Parameters<typeof createSettingsModalStore>[1] extends infer D
			? D extends { getDefaultStorageDirFn?: infer F }
				? F
				: never
			: never;
		openDialogFn?: typeof openDialog;
		/** install 成功後に 1 回呼ばれる。page-main が全 Note を Flush してから再起動する (I-SM19 / I-U8)。 */
		onUpdateInstalled?: SettingsUpdateStoreDeps['onInstalled'];
		checkForUpdatesFn?: SettingsUpdateStoreDeps['checkForUpdatesFn'];
		installableFn?: SettingsUpdateStoreDeps['installableFn'];
		installFn?: SettingsUpdateStoreDeps['installFn'];
		openUrlFn?: SettingsUpdateStoreDeps['openUrlFn'];
	};

	let {
		initial,
		onClose,
		onSaved,
		updateSettingsFn,
		getAppVersionFn,
		getDefaultStorageDirFn,
		openDialogFn = openDialog,
		onUpdateInstalled,
		checkForUpdatesFn,
		installableFn,
		installFn,
		openUrlFn
	}: Props = $props();

	// Modal は parent の {#if settingsModalOpen} で mount/unmount 制御するため、
	// initial / updateSettingsFn は mount 時の値で確定して良い（再開時は新 instance）。
	const store = untrack(() =>
		createSettingsModalStore(initial, {
			updateSettingsFn,
			getAppVersionFn,
			getDefaultStorageDirFn,
			onPreviewTheme: previewTheme
		})
	);

	const updates = untrack(() =>
		createSettingsUpdateStore({
			checkForUpdatesFn,
			installableFn,
			installFn,
			openUrlFn,
			onInstalled: onUpdateInstalled
		})
	);

	let dialogEl: HTMLDialogElement | null = $state(null);

	$effect(() => {
		dialogEl?.showModal();
	});

	// I-SM9: mount ごとに 1 回だけ取得
	$effect(() => {
		untrack(() => void store.loadAppVersion());
	});

	// I-SM12: mount ごとに 1 回だけ取得
	$effect(() => {
		untrack(() => void store.loadDefaultStorageDir());
	});

	// I-SM18: mount ごとに 1 回だけ判定 (手動確認そのものは押下時のみ、I-SM15)
	$effect(() => {
		untrack(() => void updates.loadInstallable());
	});

	const THEMES: Array<{ value: Theme; label: string }> = [
		{ value: 'System', label: 'System' },
		{ value: 'Light', label: 'Light' },
		{ value: 'Dark', label: 'Dark' }
	];

	// ---- theme preview (I-SM5) ------------------------------------------------

	function effectiveDark(theme: Theme, matches: boolean): boolean {
		if (theme === 'Dark') return true;
		if (theme === 'Light') return false;
		return matches;
	}

	function previewTheme(theme: Theme): void {
		if (typeof document === 'undefined') return;
		const mql = window.matchMedia('(prefers-color-scheme: dark)');
		const dark = effectiveDark(theme, mql.matches);
		document.documentElement.classList.toggle('dark', dark);
	}

	// ---- form handlers --------------------------------------------------------

	function pathErrorMessage(error: UpdateSettingsError): string | null {
		if (error.kind !== 'invalid_path') return null;
		return error.reason === 'not_absolute'
			? '絶対パスを指定してください'
			: '設定ファイルを含むフォルダは指定できません';
	}

	function persistErrorMessage(error: UpdateSettingsError): string | null {
		if (error.kind !== 'persist_error') return null;
		return `Failed to save: ${error.reason}`;
	}

	async function pickFolder() {
		const picked = await openDialogFn({
			directory: true,
			multiple: false,
			title: 'Select storage directory'
		});
		if (typeof picked === 'string') {
			store.setStorageDir(picked);
		}
	}

	async function handleSave(event: Event) {
		event.preventDefault();
		// I-SM20: インストール中は保存しない (Enter 等による submit も含む)
		if (updates.busy) return;
		const outcome = await store.save();
		if (outcome.kind === 'closed') {
			if (outcome.settings) onSaved?.(outcome.settings);
			onClose();
		}
	}

	function handleCancel(event: Event) {
		event.preventDefault();
		// I-SM5: cancel 時に mount 時の theme に rollback
		previewTheme(initial.theme);
		onClose();
	}

	function handleDialogCancel(event: Event) {
		// I-SM20: インストール中は Esc で閉じない
		if (updates.busy) {
			event.preventDefault();
			return;
		}
		// <dialog> の Esc → cancel event。preventDefault せず close は親に委ねる。
		// I-SM5: Esc 時も mount 時の theme に rollback
		previewTheme(initial.theme);
		onClose();
	}
</script>

<dialog
	bind:this={dialogEl}
	data-testid="widget.widget-settings-modal.root"
	aria-labelledby="widget-settings-modal-title"
	class="rounded-lg border border-neutral-200 bg-white p-0 text-neutral-900 shadow-xl backdrop:bg-transparent dark:border-neutral-800 dark:bg-neutral-900 dark:text-neutral-100"
	oncancel={handleDialogCancel}
>
	<form
		method="dialog"
		class="flex w-[28rem] max-w-[90vw] flex-col gap-4 p-5"
		onsubmit={handleSave}
	>
		<h1 id="widget-settings-modal-title" class="text-base font-semibold">Settings</h1>

		<section class="flex flex-col gap-1.5">
			<span class="text-sm font-medium">Storage directory</span>
			<div class="flex items-center gap-2">
				<input
					id="screen-2-storage-dir"
					data-testid="widget.widget-settings-modal.storage-dir"
					type="text"
					readonly
					value={store.storageDir}
					class="min-w-0 flex-1 rounded-md border border-neutral-200 bg-neutral-50 px-2 py-1 text-xs text-neutral-700 dark:border-neutral-700 dark:bg-neutral-800 dark:text-neutral-300"
					aria-label="Storage directory"
				/>
				<button
					type="button"
					class="shrink-0 rounded-md border border-neutral-200 bg-white px-2 py-1 text-xs hover:bg-neutral-100 dark:border-neutral-700 dark:bg-neutral-800 dark:hover:bg-neutral-700"
					onclick={pickFolder}
				>
					Browse…
				</button>
			</div>
			<button
				type="button"
				data-testid="widget.widget-settings-modal.storage-dir-reset"
				class="w-fit rounded-md border border-neutral-200 bg-white px-2 py-1 text-xs hover:bg-neutral-100 disabled:cursor-not-allowed disabled:opacity-60 dark:border-neutral-700 dark:bg-neutral-800 dark:hover:bg-neutral-700"
				disabled={store.defaultStorageDir === null}
				onclick={store.resetStorageDir}
			>
				Reset to default
			</button>
			{#if store.saveState.kind === 'error'}
				{@const pathMsg = pathErrorMessage(store.saveState.error)}
				{#if pathMsg}
					<p class="text-xs text-red-600 dark:text-red-400">
						{pathMsg}
					</p>
				{/if}
			{/if}
		</section>

		<section class="flex flex-col gap-1.5">
			<span class="text-sm font-medium">Theme</span>
			<div
				role="radiogroup"
				aria-label="Theme"
				data-testid="widget.widget-settings-modal.theme"
				class="inline-flex w-fit overflow-hidden rounded-md border border-neutral-200 dark:border-neutral-700"
			>
				{#each THEMES as t (t.value)}
					{@const active = store.theme === t.value}
					<button
						type="button"
						data-testid="widget.widget-settings-modal.theme-option"
						data-key={t.value}
						class={[
							'px-3 py-1 text-xs transition-colors',
							active
								? 'bg-blue-600 text-white'
								: 'bg-white text-neutral-600 hover:bg-neutral-100 dark:bg-neutral-800 dark:text-neutral-300 dark:hover:bg-neutral-700'
						]}
						aria-pressed={active}
						onclick={() => store.setTheme(t.value)}
					>
						{t.label}
					</button>
				{/each}
			</div>
		</section>

		<section
			aria-labelledby="widget-settings-modal-updates-title"
			class="flex flex-col gap-1.5 border-t border-neutral-200 pt-4 dark:border-neutral-800"
		>
			<span id="widget-settings-modal-updates-title" class="text-sm font-medium">Updates</span>
			<div class="flex items-center gap-2">
				{#if store.appVersion}
					<span
						data-testid="widget.widget-settings-modal.app-version"
						class="text-xs text-neutral-600 dark:text-neutral-400"
					>
						Version {store.appVersion}
					</span>
				{/if}
				<button
					type="button"
					data-testid="widget.widget-settings-modal.check-for-updates"
					class="ml-auto shrink-0 rounded-md border border-neutral-200 bg-white px-2 py-1 text-xs hover:bg-neutral-100 disabled:cursor-not-allowed disabled:opacity-60 dark:border-neutral-700 dark:bg-neutral-800 dark:hover:bg-neutral-700"
					disabled={updates.checkStatus.kind === 'checking' || updates.busy}
					onclick={() => void updates.check()}
				>
					Check for updates
				</button>
			</div>
			{#if updates.checkStatus.kind !== 'idle'}
				{@const status = updates.checkStatus}
				<p
					data-testid="widget.widget-settings-modal.update-check-result"
					role="status"
					class={[
						'text-xs',
						status.kind === 'failed'
							? 'text-red-600 dark:text-red-400'
							: 'text-neutral-600 dark:text-neutral-400'
					]}
				>
					{#if status.kind === 'checking'}
						Checking…
					{:else if status.kind === 'up-to-date'}
						You're up to date
					{:else if status.kind === 'available'}
						Version <span class="font-semibold">{status.release.version}</span> is available
					{:else}
						Couldn't check for updates
					{/if}
				</p>
			{/if}
			{#if updates.installStatus === 'installing'}
				<p
					data-testid="widget.widget-settings-modal.install-status"
					class="text-xs text-neutral-600 dark:text-neutral-400"
				>
					Updating…
				</p>
			{:else if updates.installStatus === 'failed'}
				<p
					data-testid="widget.widget-settings-modal.install-status"
					role="alert"
					class="text-xs text-red-600 dark:text-red-400"
				>
					Update failed. Try again or download it from the release page.
				</p>
			{:else if updates.installStatus === 'already-installing'}
				<p
					data-testid="widget.widget-settings-modal.install-status"
					class="text-xs text-neutral-600 dark:text-neutral-400"
				>
					An update is already being installed.
				</p>
			{:else if updates.installStatus === 'restart-required'}
				<p
					data-testid="widget.widget-settings-modal.install-status"
					role="alert"
					class="text-xs text-neutral-600 dark:text-neutral-400"
				>
					The update was installed. Restart PromptNotes to finish updating.
				</p>
			{/if}
			{#if updates.checkStatus.kind === 'available'}
				<div class="flex justify-end gap-1">
					<button
						type="button"
						data-testid="widget.widget-settings-modal.view-release"
						class="rounded px-2 py-1 text-xs font-medium text-blue-600 hover:bg-blue-50 dark:text-blue-400 dark:hover:bg-blue-950"
						onclick={() => void updates.viewRelease()}
					>
						View release
					</button>
					{#if updates.installable}
						<button
							type="button"
							data-testid="widget.widget-settings-modal.install-update"
							disabled={updates.busy || updates.installStatus === 'restart-required'}
							class="rounded bg-blue-600 px-2 py-1 text-xs font-medium text-white hover:bg-blue-700 disabled:cursor-not-allowed disabled:opacity-60 dark:bg-blue-500 dark:hover:bg-blue-600"
							onclick={() => void updates.install()}
						>
							Update & restart
						</button>
					{/if}
				</div>
			{/if}
		</section>

		{#if store.saveState.kind === 'error'}
			{@const persistMsg = persistErrorMessage(store.saveState.error)}
			{#if persistMsg}
				<p class="text-xs text-red-600 dark:text-red-400">
					{persistMsg}
				</p>
			{/if}
		{/if}

		<footer class="flex items-center justify-end gap-2 pt-2">
			<button
				type="button"
				data-testid="widget.widget-settings-modal.cancel"
				disabled={updates.busy}
				class="rounded-md border border-neutral-200 bg-white px-3 py-1 text-xs hover:bg-neutral-100 disabled:cursor-not-allowed disabled:opacity-60 dark:border-neutral-700 dark:bg-neutral-800 dark:hover:bg-neutral-700"
				onclick={handleCancel}
			>
				Cancel
			</button>
			<button
				type="submit"
				data-testid="widget.widget-settings-modal.save"
				disabled={store.saveState.kind === 'saving' || updates.busy}
				class="rounded-md bg-blue-600 px-3 py-1 text-xs font-medium text-white hover:bg-blue-700 disabled:cursor-not-allowed disabled:opacity-60"
			>
				Save
			</button>
		</footer>
	</form>
</dialog>
