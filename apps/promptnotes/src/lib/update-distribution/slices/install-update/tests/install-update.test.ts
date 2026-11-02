import { afterEach, describe, expect, it, vi } from 'vitest';

const invokeMock = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));

import { installUpdate, isInstallInProgressError, isUpdateInstallable, restartApp } from '../index';

describe('slice:install-update', () => {
	afterEach(() => {
		invokeMock.mockReset();
	});

	it('spec.md#tp-boundary TP-B1: installUpdate は install_update を 1 回 invoke しバージョンを返す', async () => {
		invokeMock.mockResolvedValue('0.2.5');

		await expect(installUpdate()).resolves.toBe('0.2.5');
		expect(invokeMock).toHaveBeenCalledExactlyOnceWith('install_update');
	});

	it('spec.md#tp-boundary TP-B1: installUpdate は Rust のエラー DTO をそのまま reject する', async () => {
		const error = { kind: 'DownloadFailed', message: 'failed to download the update: net' };
		invokeMock.mockRejectedValue(error);

		await expect(installUpdate()).rejects.toEqual(error);
	});

	it('spec.md#tp-boundary TP-B2: isUpdateInstallable は is_update_installable を invoke する', async () => {
		invokeMock.mockResolvedValue(false);

		await expect(isUpdateInstallable()).resolves.toBe(false);
		expect(invokeMock).toHaveBeenCalledExactlyOnceWith('is_update_installable');
	});

	it('spec.md#tp-boundary TP-B2: restartApp は restart_app を invoke する', async () => {
		invokeMock.mockResolvedValue(undefined);

		await restartApp();
		expect(invokeMock).toHaveBeenCalledExactlyOnceWith('restart_app');
	});

	it('spec.md#tp-concurrency TP-G7: isInstallInProgressError は kind=InstallInProgress だけを true にする', () => {
		expect(isInstallInProgressError({ kind: 'InstallInProgress', message: 'x' })).toBe(true);
		expect(isInstallInProgressError({ kind: 'DownloadFailed', message: 'x' })).toBe(false);
		expect(isInstallInProgressError(new Error('InstallInProgress'))).toBe(false);
		expect(isInstallInProgressError(null)).toBe(false);
		expect(isInstallInProgressError('InstallInProgress')).toBe(false);
	});
});
