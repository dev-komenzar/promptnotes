import { describe, expect, it, vi } from 'vitest';
import type {
	SettingsDto,
	UpdateSettingsError
} from '$lib/user-preferences/slices/update-settings';
import { createSettingsModalStore } from '../store.svelte';

function makeSettings(overrides: Partial<SettingsDto> = {}): SettingsDto {
	return {
		storage_dir: '/Users/foo/Documents/PromptNotes',
		theme: 'System',
		sort_preference: { field: 'created_at', direction: 'desc' },
		...overrides
	};
}

describe('widget:widget-settings-modal store', () => {
	it('spec#tp-sm-mount-defaults — mount 時の initial settings が draft に反映される', () => {
		const updateSettingsFn = vi.fn();
		const store = createSettingsModalStore(makeSettings({ theme: 'Dark' }), {
			updateSettingsFn
		});

		expect(store.storageDir).toBe('/Users/foo/Documents/PromptNotes');
		expect(store.theme).toBe('Dark');
		expect(store.dirty).toBe(false);
		expect(updateSettingsFn).not.toHaveBeenCalled();
	});

	it('spec#invariants-form-state (I-SM4) — setStorageDir で dirty=true、baseline は不変', () => {
		const store = createSettingsModalStore(makeSettings(), {
			updateSettingsFn: vi.fn()
		});

		store.setStorageDir('/new/abs');

		expect(store.storageDir).toBe('/new/abs');
		expect(store.dirty).toBe(true);
		expect(store.baseline.storage_dir).toBe('/Users/foo/Documents/PromptNotes');
	});

	it('spec#tp-sm-save-invokes-workflow — theme 変更の save で updateSettings({theme}) を 1 回呼ぶ', async () => {
		const updated = makeSettings({ theme: 'Dark' });
		const updateSettingsFn = vi.fn().mockResolvedValue(updated);
		const store = createSettingsModalStore(makeSettings(), { updateSettingsFn });

		store.setTheme('Dark');
		const outcome = await store.save();

		expect(outcome).toStrictEqual({ kind: 'closed', settings: updated });
		expect(updateSettingsFn).toHaveBeenCalledTimes(1);
		expect(updateSettingsFn).toHaveBeenCalledWith({ theme: 'Dark' });
	});

	it('spec#tp-sm-save-invokes-workflow — storage_dir + theme 両方変更で payload に両方含む', async () => {
		const updateSettingsFn = vi
			.fn()
			.mockResolvedValue(makeSettings({ storage_dir: '/x', theme: 'Light' }));
		const store = createSettingsModalStore(makeSettings(), { updateSettingsFn });

		store.setStorageDir('/x');
		store.setTheme('Light');
		await store.save();

		expect(updateSettingsFn).toHaveBeenCalledWith({ storage_dir: '/x', theme: 'Light' });
	});

	it('ori-ayx — save 成功時 outcome に updateSettings の戻り値 (新 SettingsDto) を含む', async () => {
		const updated = makeSettings({ storage_dir: '/new/storage' });
		const updateSettingsFn = vi.fn().mockResolvedValue(updated);
		const store = createSettingsModalStore(makeSettings(), { updateSettingsFn });

		store.setStorageDir('/new/storage');
		const outcome = await store.save();

		expect(outcome).toStrictEqual({ kind: 'closed', settings: updated });
	});

	it('spec#tp-sm-save-no-diff (I-SM6) — 差分なし save は updateSettings を呼ばずに closed を返す', async () => {
		const updateSettingsFn = vi.fn();
		const store = createSettingsModalStore(makeSettings(), { updateSettingsFn });

		const outcome = await store.save();

		expect(outcome).toStrictEqual({ kind: 'closed' });
		expect(updateSettingsFn).not.toHaveBeenCalled();
	});

	it('spec#tp-sm-save-error-keeps-open (I-SM3) — InvalidPath reject で error 状態 + close しない', async () => {
		const error: UpdateSettingsError = {
			kind: 'invalid_path',
			path: '/relative',
			reason: 'not_absolute'
		};
		const updateSettingsFn = vi.fn().mockRejectedValue(error);
		const store = createSettingsModalStore(makeSettings(), { updateSettingsFn });

		store.setStorageDir('/relative');
		const outcome = await store.save();

		expect(outcome).toStrictEqual({ kind: 'error', error });
		expect(store.saveState).toStrictEqual({ kind: 'error', error });
	});

	it('spec#tp-sm-storage-dir-validation — error 状態は setStorageDir で idle に戻る', async () => {
		const error: UpdateSettingsError = {
			kind: 'invalid_path',
			path: '/relative',
			reason: 'not_absolute'
		};
		const updateSettingsFn = vi.fn().mockRejectedValue(error);
		const store = createSettingsModalStore(makeSettings(), { updateSettingsFn });

		store.setStorageDir('/relative');
		await store.save();
		expect(store.saveState.kind).toBe('error');

		store.setStorageDir('/absolute/path');
		expect(store.saveState).toStrictEqual({ kind: 'idle' });
	});

	it('spec#tp-sm-save-invokes-workflow — 同値再代入は dirty=false のまま', () => {
		const store = createSettingsModalStore(makeSettings({ theme: 'System' }), {
			updateSettingsFn: vi.fn()
		});

		store.setTheme('System');

		expect(store.dirty).toBe(false);
	});

	it('spec#I-SM5 — setTheme で onPreviewTheme callback が呼ばれる', () => {
		const onPreviewTheme = vi.fn();
		const store = createSettingsModalStore(makeSettings({ theme: 'System' }), {
			updateSettingsFn: vi.fn(),
			onPreviewTheme
		});

		store.setTheme('Dark');

		expect(onPreviewTheme).toHaveBeenCalledTimes(1);
		expect(onPreviewTheme).toHaveBeenCalledWith('Dark');
	});

	it('spec#I-SM5 — Light → Dark → System と連続選択で preview が毎回呼ばれる', () => {
		const onPreviewTheme = vi.fn();
		const store = createSettingsModalStore(makeSettings({ theme: 'System' }), {
			updateSettingsFn: vi.fn(),
			onPreviewTheme
		});

		store.setTheme('Light');
		store.setTheme('Dark');
		store.setTheme('System');

		expect(onPreviewTheme).toHaveBeenCalledTimes(3);
		expect(onPreviewTheme.mock.calls).toEqual([['Light'], ['Dark'], ['System']]);
	});

	it('spec#tp-sm-app-version-shown — loadAppVersion で getAppVersion を 1 回呼び appVersion に反映', async () => {
		const getAppVersionFn = vi.fn().mockResolvedValue('0.2.2');
		const store = createSettingsModalStore(makeSettings(), {
			updateSettingsFn: vi.fn(),
			getAppVersionFn
		});

		await store.loadAppVersion();

		expect(getAppVersionFn).toHaveBeenCalledTimes(1);
		expect(store.appVersion).toBe('0.2.2');
	});

	it('spec#tp-sm-app-version-hidden-before-resolve — 取得完了前の appVersion は null', () => {
		const getAppVersionFn = vi.fn().mockReturnValue(new Promise<string>(() => {}));
		const store = createSettingsModalStore(makeSettings(), {
			updateSettingsFn: vi.fn(),
			getAppVersionFn
		});

		void store.loadAppVersion();

		expect(store.appVersion).toBeNull();
	});

	it('spec#tp-sm-app-version-hidden-on-error — reject 時は appVersion=null のまま例外を漏らさない', async () => {
		const getAppVersionFn = vi.fn().mockRejectedValue(new Error('ipc failed'));
		const store = createSettingsModalStore(makeSettings(), {
			updateSettingsFn: vi.fn(),
			getAppVersionFn
		});

		await expect(store.loadAppVersion()).resolves.toBeUndefined();

		expect(store.appVersion).toBeNull();
		expect(store.saveState).toStrictEqual({ kind: 'idle' });
	});

	it('spec#tp-sm-app-version-not-in-diff — バージョン取得後の無編集 save は updateSettings を呼ばず close', async () => {
		const updateSettingsFn = vi.fn();
		const store = createSettingsModalStore(makeSettings(), {
			updateSettingsFn,
			getAppVersionFn: vi.fn().mockResolvedValue('0.2.2')
		});

		await store.loadAppVersion();
		const outcome = await store.save();

		expect(store.dirty).toBe(false);
		expect(outcome).toStrictEqual({ kind: 'closed' });
		expect(updateSettingsFn).not.toHaveBeenCalled();
	});
});
