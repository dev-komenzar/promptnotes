import { describe, expect, it, vi } from 'vitest';
import type { UpdateChannelResponse } from '$lib/update-distribution/slices/check-for-updates-manually';
import { createSettingsUpdateStore } from '../update-store.svelte';

const RELEASE_URL = 'https://github.com/dev-komenzar/promptnotes/releases/tag/v0.2.5';

function available(version = '0.2.5'): UpdateChannelResponse {
	return {
		current_version: '0.2.4',
		latest_release: { version, url: RELEASE_URL, notes: 'notes' }
	};
}

function upToDate(): UpdateChannelResponse {
	return { current_version: '0.2.5', latest_release: null };
}

function deferred<T>() {
	let resolve!: (value: T) => void;
	let reject!: (reason: unknown) => void;
	const promise = new Promise<T>((res, rej) => {
		resolve = res;
		reject = rej;
	});
	return { promise, resolve, reject };
}

/** 新バージョンを検出済み + installable の store を作る。 */
async function availableStore(deps: Parameters<typeof createSettingsUpdateStore>[0] = {}) {
	const store = createSettingsUpdateStore({
		checkForUpdatesFn: vi.fn().mockResolvedValue(available()),
		installableFn: vi.fn().mockResolvedValue(true),
		...deps
	});
	await store.loadInstallable();
	await store.check();
	return store;
}

describe('widget:widget-settings-modal update store', () => {
	describe('spec#invariants-updates 手動確認', () => {
		it('spec#tp-sm-update-check-on-click (I-SM15) — 生成直後は確認せず、check() で 1 回呼ぶ', async () => {
			const checkForUpdatesFn = vi.fn().mockResolvedValue(upToDate());
			const store = createSettingsUpdateStore({ checkForUpdatesFn });

			expect(store.checkStatus).toEqual({ kind: 'idle' });
			expect(checkForUpdatesFn).not.toHaveBeenCalled();

			await store.check();

			expect(checkForUpdatesFn).toHaveBeenCalledTimes(1);
		});

		it('spec#tp-sm-update-checking-disabled (I-SM16) — 確認中は checking、重ねて呼んでも 1 回だけ', async () => {
			const d = deferred<UpdateChannelResponse>();
			const checkForUpdatesFn = vi.fn().mockReturnValue(d.promise);
			const store = createSettingsUpdateStore({ checkForUpdatesFn });

			const first = store.check();
			expect(store.checkStatus).toEqual({ kind: 'checking' });
			await store.check();
			expect(checkForUpdatesFn).toHaveBeenCalledTimes(1);

			d.resolve(upToDate());
			await first;
			expect(store.checkStatus).toEqual({ kind: 'up-to-date' });
		});

		it('spec#tp-sm-update-up-to-date (I-SM17) — latest_release=null は up-to-date', async () => {
			const store = createSettingsUpdateStore({
				checkForUpdatesFn: vi.fn().mockResolvedValue(upToDate())
			});

			await store.check();

			expect(store.checkStatus).toEqual({ kind: 'up-to-date' });
		});

		it('spec#tp-sm-update-available (I-SM17) — latest_release ありは available + release を保持', async () => {
			const store = createSettingsUpdateStore({
				checkForUpdatesFn: vi.fn().mockResolvedValue(available('0.2.5'))
			});

			await store.check();

			expect(store.checkStatus).toEqual({
				kind: 'available',
				release: { version: '0.2.5', url: RELEASE_URL, notes: 'notes' }
			});
		});

		it('spec#tp-sm-update-check-failed (I-SM17) — reject は failed、例外は漏らさず再確認できる', async () => {
			const checkForUpdatesFn = vi
				.fn()
				.mockRejectedValueOnce({ kind: 'NetworkError', message: 'offline' })
				.mockResolvedValueOnce(upToDate());
			const store = createSettingsUpdateStore({ checkForUpdatesFn });

			await expect(store.check()).resolves.toBeUndefined();
			expect(store.checkStatus).toEqual({ kind: 'failed' });

			await store.check();
			expect(store.checkStatus).toEqual({ kind: 'up-to-date' });
			expect(checkForUpdatesFn).toHaveBeenCalledTimes(2);
		});
	});

	describe('spec#invariants-updates インストール操作の提示', () => {
		it('spec#tp-sm-update-available (I-SM18) — loadInstallable で isUpdateInstallable を反映', async () => {
			const installableFn = vi.fn().mockResolvedValue(true);
			const store = createSettingsUpdateStore({ installableFn });

			expect(store.installable).toBe(false);
			await store.loadInstallable();

			expect(store.installable).toBe(true);
			expect(installableFn).toHaveBeenCalledTimes(1);
		});

		it('spec#tp-sm-update-available (I-SM18) — isUpdateInstallable の reject は false', async () => {
			const store = createSettingsUpdateStore({
				installableFn: vi.fn().mockRejectedValue(new Error('ipc'))
			});

			await expect(store.loadInstallable()).resolves.toBeUndefined();
			expect(store.installable).toBe(false);
		});

		it('spec#tp-sm-update-available — viewRelease は release.url を openUrlFn で開く', async () => {
			const openUrlFn = vi.fn();
			const store = await availableStore({ openUrlFn });

			await store.viewRelease();

			expect(openUrlFn).toHaveBeenCalledExactlyOnceWith(RELEASE_URL);
		});

		it('viewRelease は新バージョンが無ければ no-op', async () => {
			const openUrlFn = vi.fn();
			const store = createSettingsUpdateStore({
				checkForUpdatesFn: vi.fn().mockResolvedValue(upToDate()),
				openUrlFn
			});
			await store.check();

			await store.viewRelease();

			expect(openUrlFn).not.toHaveBeenCalled();
		});
	});

	describe('spec#invariants-updates インストール', () => {
		it('spec#tp-sm-update-install-success (I-SM19 / I-SM20) — install 中は installing、成功後に onInstalled', async () => {
			const d = deferred<string>();
			const installFn = vi.fn().mockReturnValue(d.promise);
			const onInstalled = vi.fn();
			const store = await availableStore({ installFn, onInstalled });

			const pending = store.install();
			expect(store.installStatus).toBe('installing');
			expect(store.busy).toBe(true);
			expect(onInstalled).not.toHaveBeenCalled();

			d.resolve('0.2.5');
			await pending;

			expect(installFn).toHaveBeenCalledTimes(1);
			expect(onInstalled).toHaveBeenCalledExactlyOnceWith('0.2.5');
			// 再起動までは installing のまま (操作を受け付けない)
			expect(store.installStatus).toBe('installing');
		});

		it('spec#tp-sm-update-install-success (I-SM20) — install 中の install / check は no-op', async () => {
			const d = deferred<string>();
			const installFn = vi.fn().mockReturnValue(d.promise);
			const checkForUpdatesFn = vi.fn().mockResolvedValue(available());
			const store = await availableStore({ installFn, checkForUpdatesFn });

			const pending = store.install();
			await store.install();
			await store.check();

			expect(installFn).toHaveBeenCalledTimes(1);
			expect(checkForUpdatesFn).toHaveBeenCalledTimes(1);
			d.resolve('0.2.5');
			await pending;
		});

		it('spec#tp-sm-update-install-failed (I-SM21) — reject は failed、onInstalled は呼ばず再試行できる', async () => {
			const installFn = vi
				.fn()
				.mockRejectedValueOnce({ kind: 'DownloadFailed', message: 'net' })
				.mockResolvedValueOnce('0.2.5');
			const onInstalled = vi.fn();
			const store = await availableStore({ installFn, onInstalled });

			await store.install();
			expect(store.installStatus).toBe('failed');
			expect(store.busy).toBe(false);
			expect(onInstalled).not.toHaveBeenCalled();
			// 確認結果は維持する
			expect(store.checkStatus.kind).toBe('available');

			await store.install();
			expect(installFn).toHaveBeenCalledTimes(2);
			expect(onInstalled).toHaveBeenCalledExactlyOnceWith('0.2.5');
		});

		it('spec#invariants-updates (I-SM21) — 再確認でインストール失敗の表示を消す', async () => {
			const store = await availableStore({
				installFn: vi.fn().mockRejectedValue({ kind: 'InstallFailed', message: 'x' })
			});
			await store.install();
			expect(store.installStatus).toBe('failed');

			await store.check();

			expect(store.installStatus).toBe('idle');
		});

		it('spec#invariants-updates (I-SM18) — installable=false なら install は no-op', async () => {
			const installFn = vi.fn();
			const store = await availableStore({
				installableFn: vi.fn().mockResolvedValue(false),
				installFn
			});

			await store.install();

			expect(installFn).not.toHaveBeenCalled();
			expect(store.installStatus).toBe('idle');
		});

		it('新バージョン未検出なら install は no-op', async () => {
			const installFn = vi.fn();
			const store = createSettingsUpdateStore({
				checkForUpdatesFn: vi.fn().mockResolvedValue(upToDate()),
				installableFn: vi.fn().mockResolvedValue(true),
				installFn
			});
			await store.loadInstallable();
			await store.check();

			await store.install();

			expect(installFn).not.toHaveBeenCalled();
		});
	});
});
