import {
	updateSettings as defaultUpdateSettings,
	type SettingsDto,
	type Theme,
	type UpdateSettingsError,
	type UpdateSettingsInput
} from '$lib/user-preferences/slices/update-settings';

/**
 * widget-settings-modal の draft state + save/cancel ロジックを保持する store。
 *
 * - I-SM4: 編集中の値は modal scope に閉じる（cancel / Esc で破棄）
 * - I-SM3: save は `update-settings` slice を呼び、成功で `onSaved` を発火
 * - I-SM6: 差分なし save は workflow を呼ばずに close（C-US1 最適化）
 */

export type SettingsModalStoreDeps = {
	updateSettingsFn?: typeof defaultUpdateSettings;
	/** I-SM5: theme 選択時の即時 preview callback（DOM 操作の注入点） */
	onPreviewTheme?: (theme: Theme) => void;
};

export type SaveState =
	| { kind: 'idle' }
	| { kind: 'saving' }
	| { kind: 'error'; error: UpdateSettingsError };

export type SettingsModalStore = ReturnType<typeof createSettingsModalStore>;

export function createSettingsModalStore(initial: SettingsDto, deps: SettingsModalStoreDeps = {}) {
	const updateSettingsFn = deps.updateSettingsFn ?? defaultUpdateSettings;

	const baseline = $state<SettingsDto>({ ...initial });
	let storageDir = $state(initial.storage_dir);
	let theme = $state<Theme>(initial.theme);
	let saveState = $state<SaveState>({ kind: 'idle' });

	const dirty = $derived(storageDir !== baseline.storage_dir || theme !== baseline.theme);

	function setStorageDir(next: string): void {
		storageDir = next;
		if (saveState.kind === 'error') saveState = { kind: 'idle' };
	}

	function setTheme(next: Theme): void {
		theme = next;
		if (saveState.kind === 'error') saveState = { kind: 'idle' };
		deps.onPreviewTheme?.(next);
	}

	function buildInput(): UpdateSettingsInput {
		const input: UpdateSettingsInput = {};
		if (storageDir !== baseline.storage_dir) input.storage_dir = storageDir;
		if (theme !== baseline.theme) input.theme = theme;
		return input;
	}

	async function save(): Promise<
		| { kind: 'closed'; settings?: SettingsDto }
		| { kind: 'error'; error: UpdateSettingsError }
	> {
		// I-SM6: diff-less save short-circuits to close (skip Tauri round-trip).
		if (!dirty) {
			return { kind: 'closed' };
		}
		saveState = { kind: 'saving' };
		try {
			const settings = await updateSettingsFn(buildInput());
			saveState = { kind: 'idle' };
			return { kind: 'closed', settings };
		} catch (raw) {
			const error = raw as UpdateSettingsError;
			saveState = { kind: 'error', error };
			return { kind: 'error', error };
		}
	}

	return {
		get storageDir() {
			return storageDir;
		},
		get theme() {
			return theme;
		},
		get baseline() {
			return baseline;
		},
		get saveState() {
			return saveState;
		},
		get dirty() {
			return dirty;
		},
		setStorageDir,
		setTheme,
		save
	};
}
