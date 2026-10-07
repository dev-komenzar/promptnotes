import { afterEach, describe, expect, it, vi } from 'vitest';

const invokeMock = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));

import { getAppVersion } from '../index';

describe('slice:get-app-version', () => {
	afterEach(() => {
		invokeMock.mockReset();
	});

	it('spec.md#tp-boundary: TP-B3 getAppVersion は get_app_version を引数なしで 1 回 invoke し値を返す', async () => {
		invokeMock.mockResolvedValue('0.2.2');

		const version = await getAppVersion();

		expect(invokeMock).toHaveBeenCalledTimes(1);
		expect(invokeMock).toHaveBeenCalledWith('get_app_version');
		expect(version).toBe('0.2.2');
	});
});
