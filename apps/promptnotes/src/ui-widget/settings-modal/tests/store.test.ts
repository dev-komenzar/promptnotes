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
		const updateSettingsFn = vi.fn().mockResolvedValue(makeSettings({ theme: 'Dark' }));
		const store = createSettingsModalStore(makeSettings(), { updateSettingsFn });

		store.setTheme('Dark');
		const outcome = await store.save();

		expect(outcome).toStrictEqual({ kind: 'closed' });
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
});
