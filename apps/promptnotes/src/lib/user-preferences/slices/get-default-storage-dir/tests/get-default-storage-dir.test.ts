import { afterEach, describe, expect, it, vi } from 'vitest';

const invokeMock = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));

import { getDefaultStorageDir } from '../index';

describe('slice:get-default-storage-dir', () => {
	afterEach(() => {
		invokeMock.mockReset();
	});

	it('spec.md#tp-boundary: TP-B1 getDefaultStorageDir は get_default_storage_dir を引数なしで 1 回 invoke し値を返す', async () => {
		invokeMock.mockResolvedValue('/home/u/.local/share/promptnotes/notes');

		const dir = await getDefaultStorageDir();

		expect(invokeMock).toHaveBeenCalledTimes(1);
		expect(invokeMock).toHaveBeenCalledWith('get_default_storage_dir');
		expect(dir).toBe('/home/u/.local/share/promptnotes/notes');
	});
});
