import { describe, expect, it, vi } from 'vitest';
import { page } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import type { SettingsDto } from '$lib/user-preferences/slices/update-settings';
import type { SettingsUpdateStoreDeps } from '../update-store.svelte';

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

describe('widget:widget-settings-modal spec#tp-sm-storage-dir-validation', () => {
	async function saveRejectedWith(error: unknown) {
		const updateSettingsFn = vi.fn().mockRejectedValue(error);
		render(WidgetSettingsModal, {
			initial: makeSettings(),
			onClose: vi.fn(),
			updateSettingsFn,
			getAppVersionFn: vi.fn().mockResolvedValue('0.2.2'),
			getDefaultStorageDirFn: vi.fn().mockResolvedValue(DEFAULT_DIR)
		});
		// dirty にして diff-less short-circuit (I-SM6) を避ける
		await expect.element(resetButton()).toBeEnabled();
		await resetButton().click();
		await page.getByRole('button', { name: 'Save' }).click();
		expect(updateSettingsFn).toHaveBeenCalledTimes(1);
	}

	it('invalid_path / not_absolute — 「絶対パスを指定してください」を表示する', async () => {
		await saveRejectedWith({ kind: 'invalid_path', path: 'relative', reason: 'not_absolute' });

		await expect.element(page.getByText('絶対パスを指定してください')).toBeVisible();
	});

	it('invalid_path / contains_config_path — 設定ファイルを含むフォルダのエラーを表示する', async () => {
		await saveRejectedWith({
			kind: 'invalid_path',
			path: '/cfg/x',
			reason: 'contains_config_path'
		});

		await expect
			.element(page.getByText('設定ファイルを含むフォルダは指定できません'))
			.toBeVisible();
	});
});

describe('widget:widget-settings-modal spec#invariants-updates', () => {
	const RELEASE_URL = 'https://github.com/dev-komenzar/promptnotes/releases/tag/v0.2.5';
	const AVAILABLE = {
		current_version: '0.2.4',
		latest_release: { version: '0.2.5', url: RELEASE_URL, notes: 'notes' }
	};

	type UpdateDeps = SettingsUpdateStoreDeps & {
		onClose?: () => void;
		updateSettingsFn?: () => Promise<SettingsDto>;
		onUpdateInstalled?: SettingsUpdateStoreDeps['onInstalled'];
	};

	function renderWithUpdates(deps: UpdateDeps = {}) {
		return render(WidgetSettingsModal, {
			initial: makeSettings(),
			onClose: deps.onClose ?? vi.fn(),
			updateSettingsFn: deps.updateSettingsFn ?? vi.fn(),
			getAppVersionFn: vi.fn().mockResolvedValue('0.2.4'),
			getDefaultStorageDirFn: vi.fn().mockResolvedValue(DEFAULT_DIR),
			checkForUpdatesFn: deps.checkForUpdatesFn ?? vi.fn().mockResolvedValue(AVAILABLE),
			installableFn: deps.installableFn ?? vi.fn().mockResolvedValue(true),
			installFn: deps.installFn ?? vi.fn().mockResolvedValue('0.2.5'),
			openUrlFn: deps.openUrlFn ?? vi.fn(),
			onUpdateInstalled: deps.onUpdateInstalled ?? vi.fn()
		});
	}

	const checkButton = () => page.getByRole('button', { name: 'Check for updates' });
	const result = () => page.getByTestId('widget.widget-settings-modal.update-check-result');
	const installButton = () => page.getByRole('button', { name: 'Update & restart' });

	it('spec#cross-updates-section-layout — Updates セクションに Version を表示し、footer には出さない', async () => {
		renderWithUpdates();

		const version = page.getByTestId('widget.widget-settings-modal.app-version');
		await expect.element(version).toHaveTextContent('Version 0.2.4');
		const section = page.getByRole('region', { name: 'Updates' });
		await expect.element(section).toBeInTheDocument();
		expect(section.element().contains(version.element())).toBe(true);
		expect(document.querySelector('footer')?.textContent).not.toContain('0.2.4');
	});

	it('spec#tp-sm-update-check-on-click (I-SM15) — mount 時は確認せず、押下で 1 回確認する', async () => {
		const checkForUpdatesFn = vi.fn().mockResolvedValue(AVAILABLE);
		renderWithUpdates({ checkForUpdatesFn });

		await expect.element(checkButton()).toBeEnabled();
		expect(checkForUpdatesFn).not.toHaveBeenCalled();
		expect(
			document.querySelector('[data-testid="widget.widget-settings-modal.update-check-result"]')
		).toBeNull();

		await checkButton().click();

		await expect.element(result()).toHaveTextContent('Version 0.2.5 is available');
		expect(checkForUpdatesFn).toHaveBeenCalledTimes(1);
	});

	it('spec#tp-sm-update-checking-disabled (I-SM16) — 確認中は Checking… でボタンが disabled', async () => {
		renderWithUpdates({ checkForUpdatesFn: vi.fn().mockReturnValue(new Promise(() => {})) });

		await checkButton().click();

		await expect.element(result()).toHaveTextContent('Checking…');
		await expect.element(checkButton()).toBeDisabled();
	});

	it("spec#tp-sm-update-up-to-date (I-SM17) — 最新なら You're up to date、操作ボタンは出さない", async () => {
		renderWithUpdates({
			checkForUpdatesFn: vi
				.fn()
				.mockResolvedValue({ current_version: '0.2.5', latest_release: null })
		});

		await checkButton().click();

		await expect.element(result()).toHaveTextContent("You're up to date");
		expect(
			document.querySelector('[data-testid="widget.widget-settings-modal.view-release"]')
		).toBeNull();
		expect(
			document.querySelector('[data-testid="widget.widget-settings-modal.install-update"]')
		).toBeNull();
	});

	it("spec#tp-sm-update-check-failed (I-SM17) — 失敗は Couldn't check for updates、再度押せる", async () => {
		vi.spyOn(console, 'error').mockImplementation(() => {});
		renderWithUpdates({
			checkForUpdatesFn: vi.fn().mockRejectedValue({ kind: 'NetworkError', message: 'offline' })
		});

		await checkButton().click();

		await expect.element(result()).toHaveTextContent("Couldn't check for updates");
		await expect.element(checkButton()).toBeEnabled();
	});

	it('spec#tp-sm-update-available (I-SM18) — View release で release URL を開く', async () => {
		const openUrlFn = vi.fn();
		renderWithUpdates({ openUrlFn });

		await checkButton().click();
		await page.getByRole('button', { name: 'View release' }).click();

		expect(openUrlFn).toHaveBeenCalledExactlyOnceWith(RELEASE_URL);
		await expect.element(installButton()).toBeInTheDocument();
	});

	it('spec#tp-sm-update-available (I-SM18 / I-U6) — installable=false なら Update & restart を出さない', async () => {
		renderWithUpdates({ installableFn: vi.fn().mockResolvedValue(false) });

		await checkButton().click();

		await expect.element(page.getByRole('button', { name: 'View release' })).toBeInTheDocument();
		expect(
			document.querySelector('[data-testid="widget.widget-settings-modal.install-update"]')
		).toBeNull();
	});

	it('spec#tp-sm-update-install-success (I-SM19 / I-SM20) — install 中は操作不可、成功で onUpdateInstalled', async () => {
		let resolveInstall!: (v: string) => void;
		const installFn = vi.fn().mockReturnValue(new Promise<string>((r) => (resolveInstall = r)));
		const onUpdateInstalled = vi.fn();
		renderWithUpdates({ installFn, onUpdateInstalled });

		await checkButton().click();
		await installButton().click();

		await expect
			.element(page.getByTestId('widget.widget-settings-modal.install-status'))
			.toHaveTextContent('Updating…');
		await expect.element(installButton()).toBeDisabled();
		await expect.element(checkButton()).toBeDisabled();
		await expect.element(page.getByRole('button', { name: 'Save' })).toBeDisabled();
		await expect.element(page.getByRole('button', { name: 'Cancel' })).toBeDisabled();
		expect(onUpdateInstalled).not.toHaveBeenCalled();

		resolveInstall('0.2.5');

		await expect.poll(() => onUpdateInstalled.mock.calls.length).toBe(1);
		expect(onUpdateInstalled).toHaveBeenCalledWith('0.2.5');
		expect(installFn).toHaveBeenCalledTimes(1);
	});

	it('spec#tp-sm-update-install-blocks-close (I-SM20) — install 中は Esc で閉じない', async () => {
		const onClose = vi.fn();
		renderWithUpdates({ onClose, installFn: vi.fn().mockReturnValue(new Promise(() => {})) });

		await checkButton().click();
		await installButton().click();
		await expect
			.element(page.getByTestId('widget.widget-settings-modal.install-status'))
			.toHaveTextContent('Updating…');

		const dialog = page.getByTestId('widget.widget-settings-modal.root').element();
		dialog.dispatchEvent(new Event('cancel', { cancelable: true }));

		expect(onClose).not.toHaveBeenCalled();
	});

	it('spec#tp-sm-update-install-failed (I-SM21) — 失敗を表示し、再試行できる', async () => {
		vi.spyOn(console, 'error').mockImplementation(() => {});
		const installFn = vi
			.fn()
			.mockRejectedValueOnce({ kind: 'DownloadFailed', message: 'net' })
			.mockResolvedValueOnce('0.2.5');
		const onUpdateInstalled = vi.fn();
		renderWithUpdates({ installFn, onUpdateInstalled });

		await checkButton().click();
		await installButton().click();

		await expect
			.element(page.getByTestId('widget.widget-settings-modal.install-status'))
			.toHaveTextContent('Update failed');
		expect(onUpdateInstalled).not.toHaveBeenCalled();
		await expect.element(installButton()).toBeEnabled();

		await installButton().click();
		await expect.poll(() => onUpdateInstalled.mock.calls.length).toBe(1);
		expect(installFn).toHaveBeenCalledTimes(2);
	});

	it('spec#tp-sm-update-not-in-diff (I-SM17) — 確認後の無編集 save は updateSettings を呼ばず close', async () => {
		const onClose = vi.fn();
		const updateSettingsFn = vi.fn();
		renderWithUpdates({ onClose, updateSettingsFn });

		await checkButton().click();
		await expect.element(result()).toHaveTextContent('Version 0.2.5 is available');
		await page.getByRole('button', { name: 'Save' }).click();

		await expect.poll(() => onClose.mock.calls.length).toBe(1);
		expect(updateSettingsFn).not.toHaveBeenCalled();
	});
});
