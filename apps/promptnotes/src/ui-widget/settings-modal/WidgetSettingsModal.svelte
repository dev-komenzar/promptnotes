<script lang="ts">
	import { untrack } from 'svelte';
	import { open as openDialog } from '@tauri-apps/plugin-dialog';
	import type {
		SettingsDto,
		Theme,
		UpdateSettingsError
	} from '$lib/user-preferences/slices/update-settings';
	import { createSettingsModalStore } from './store.svelte';

	type Props = {
		initial: SettingsDto;
		onClose: () => void;
		onSaved?: (settings: SettingsDto) => void;
		updateSettingsFn?: Parameters<typeof createSettingsModalStore>[1] extends infer D
			? D extends { updateSettingsFn?: infer F }
				? F
				: never
			: never;
		openDialogFn?: typeof openDialog;
	};

	let { initial, onClose, onSaved, updateSettingsFn, openDialogFn = openDialog }: Props = $props();

	// Modal は parent の {#if settingsModalOpen} で mount/unmount 制御するため、
	// initial / updateSettingsFn は mount 時の値で確定して良い（再開時は新 instance）。
	const store = untrack(() =>
		createSettingsModalStore(initial, { updateSettingsFn, onPreviewTheme: previewTheme })
	);

	let dialogEl: HTMLDialogElement | null = $state(null);

	$effect(() => {
		dialogEl?.showModal();
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
			? 'Path must be absolute.'
			: 'Path inside the config directory is not allowed.';
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

	function handleDialogCancel() {
		// <dialog> の Esc → cancel event。preventDefault せず close は親に委ねる。
		// I-SM5: Esc 時も mount 時の theme に rollback
		previewTheme(initial.theme);
		onClose();
	}
</script>

<dialog
	bind:this={dialogEl}
	data-testid="widget-settings-modal"
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
					data-testid="screen-2-storage-dir"
					type="text"
					readonly
					value={store.storageDir}
					class="min-w-0 flex-1 rounded-md border border-neutral-200 bg-neutral-50 px-2 py-1 text-xs text-neutral-700 dark:border-neutral-700 dark:bg-neutral-800 dark:text-neutral-300"
					aria-label="Storage directory"
				/>
				<button
					type="button"
					data-testid="screen-2-storage-dir-pick"
					class="shrink-0 rounded-md border border-neutral-200 bg-white px-2 py-1 text-xs hover:bg-neutral-100 dark:border-neutral-700 dark:bg-neutral-800 dark:hover:bg-neutral-700"
					onclick={pickFolder}
				>
					Browse…
				</button>
			</div>
			{#if store.saveState.kind === 'error'}
				{@const pathMsg = pathErrorMessage(store.saveState.error)}
				{#if pathMsg}
					<p
						data-testid="screen-2-storage-dir-error"
						class="text-xs text-red-600 dark:text-red-400"
					>
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
				data-testid="screen-2-theme"
				class="inline-flex w-fit overflow-hidden rounded-md border border-neutral-200 dark:border-neutral-700"
			>
				{#each THEMES as t (t.value)}
					{@const active = store.theme === t.value}
					<button
						type="button"
						data-testid={`screen-2-theme-${t.value}`}
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

		{#if store.saveState.kind === 'error'}
			{@const persistMsg = persistErrorMessage(store.saveState.error)}
			{#if persistMsg}
				<p
					data-testid="widget-settings-modal-persist-error"
					class="text-xs text-red-600 dark:text-red-400"
				>
					{persistMsg}
				</p>
			{/if}
		{/if}

		<footer class="flex justify-end gap-2 pt-2">
			<button
				type="button"
				data-testid="screen-2-cancel"
				class="rounded-md border border-neutral-200 bg-white px-3 py-1 text-xs hover:bg-neutral-100 dark:border-neutral-700 dark:bg-neutral-800 dark:hover:bg-neutral-700"
				onclick={handleCancel}
			>
				Cancel
			</button>
			<button
				type="submit"
				data-testid="screen-2-save"
				disabled={store.saveState.kind === 'saving'}
				class="rounded-md bg-blue-600 px-3 py-1 text-xs font-medium text-white hover:bg-blue-700 disabled:cursor-not-allowed disabled:opacity-60"
			>
				Save
			</button>
		</footer>
	</form>
</dialog>
