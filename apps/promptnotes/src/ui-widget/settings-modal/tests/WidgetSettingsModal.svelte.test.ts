import { describe, expect, it, vi } from 'vitest';
import { page } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import type { SettingsDto } from '$lib/user-preferences/slices/update-settings';

const { default: WidgetSettingsModal } = await import('../WidgetSettingsModal.svelte');

const DEFAULT_DIR = '/home/foo/.local/share/promptnotes/notes';

function makeSettings(): SettingsDto {
	return {
		storage_dir: '/Users/foo/Documents/PromptNotes',
		theme: 'System',
		sort_preference: { field: 'created_at', direction: 'desc' }
	};
}

function renderModal(getDefaultStorageDirFn: () => Promise<string>) {
	return render(WidgetSettingsModal, {
		initial: makeSettings(),
		onClose: vi.fn(),
		updateSettingsFn: vi.fn(),
		getAppVersionFn: vi.fn().mockResolvedValue('0.2.2'),
		getDefaultStorageDirFn
	});
}

const resetButton = () => page.getByRole('button', { name: 'Reset to default' });
const storageDirInput = () => page.getByRole('textbox', { name: 'Storage directory' });

describe('widget:widget-settings-modal spec#invariants-storage-dir-reset', () => {
	it('spec#tp-sm-reset-sets-draft (I-SM12) — default 取得後に押すと storage_dir 表示が default に変わる', async () => {
		const getDefaultStorageDirFn = vi.fn().mockResolvedValue(DEFAULT_DIR);
		renderModal(getDefaultStorageDirFn);

		await expect.element(resetButton()).toBeEnabled();
		await resetButton().click();

		await expect.element(storageDirInput()).toHaveValue(DEFAULT_DIR);
		expect(getDefaultStorageDirFn).toHaveBeenCalledTimes(1);
	});

	it('spec#tp-sm-reset-disabled-on-error (I-SM13) — default 取得失敗時は button が disabled', async () => {
		renderModal(vi.fn().mockRejectedValue(new Error('ipc failed')));

		await expect.element(resetButton()).toBeDisabled();
		await expect.element(storageDirInput()).toHaveValue('/Users/foo/Documents/PromptNotes');
	});

	it('spec#tp-sm-reset-disabled-before-resolve (I-SM13) — default 取得完了前は button が disabled', async () => {
		renderModal(() => new Promise<string>(() => {}));

		await expect.element(resetButton()).toBeDisabled();
	});
});
