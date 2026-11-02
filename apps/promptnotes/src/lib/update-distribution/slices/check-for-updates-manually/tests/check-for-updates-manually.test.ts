import { afterEach, describe, expect, it, vi } from 'vitest';

const invokeMock = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));

import { checkForUpdatesManually } from '../index';

describe('slice:check-for-updates-manually', () => {
	afterEach(() => {
		invokeMock.mockReset();
	});

	it('spec.md#tp-boundary TP-B1: check_for_updates_manually を 1 回 invoke し確認結果を返す', async () => {
		const response = {
			current_version: '0.2.4',
			latest_release: {
				version: '0.2.5',
				url: 'https://github.com/dev-komenzar/promptnotes/releases/tag/v0.2.5',
				notes: 'notes'
			}
		};
		invokeMock.mockResolvedValue(response);

		await expect(checkForUpdatesManually()).resolves.toEqual(response);
		expect(invokeMock).toHaveBeenCalledExactlyOnceWith('check_for_updates_manually');
	});

	it('spec.md#tp-boundary TP-B1: Rust のエラー DTO をそのまま reject する', async () => {
		const error = { kind: 'NetworkError', message: 'network error while fetching latest release' };
		invokeMock.mockRejectedValue(error);

		await expect(checkForUpdatesManually()).rejects.toEqual(error);
	});
});
