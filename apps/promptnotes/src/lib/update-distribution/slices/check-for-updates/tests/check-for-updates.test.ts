import { afterEach, describe, expect, it, vi } from 'vitest';

const invokeMock = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));

import { checkForUpdates } from '../index';

describe('slice:check-for-updates', () => {
	afterEach(() => {
		invokeMock.mockReset();
	});

	it('spec.md#impl-layers: checkForUpdates は check_for_updates を引数なしで 1 回 invoke し応答を返す', async () => {
		const response = {
			current_version: '0.2.2',
			latest_release: {
				version: '0.2.3',
				url: 'https://github.com/dev-komenzar/promptnotes/releases/tag/v0.2.3',
				notes: ''
			}
		};
		invokeMock.mockResolvedValue(response);

		const result = await checkForUpdates();

		expect(invokeMock).toHaveBeenCalledExactlyOnceWith('check_for_updates');
		expect(result).toEqual(response);
	});
});
