import { describe, expect, it, vi } from 'vitest';
import { createPendingFlushRegistry } from './pending-flush.svelte';

describe('page:page-main pending-flush registry (ori-73q / spec#impl-quit-orchestration)', () => {
	it('register → flushAll で登録済 callback を app_quit で呼ぶ', async () => {
		const reg = createPendingFlushRegistry();
		const fa = vi.fn().mockResolvedValue(undefined);
		reg.register('a', fa);

		await reg.flushAll('app_quit');

		expect(fa).toHaveBeenCalledTimes(1);
		expect(fa).toHaveBeenCalledWith('app_quit');
	});

	it('S13 — 複数 entry を挿入順に順次 await する (並列化しない)', async () => {
		const reg = createPendingFlushRegistry();
		const order: string[] = [];
		const make = (id: string) =>
			vi.fn().mockImplementation(async () => {
				order.push(`${id}:start`);
				await Promise.resolve();
				await Promise.resolve();
				order.push(`${id}:end`);
			});
		reg.register('A', make('A'));
		reg.register('B', make('B'));
		reg.register('C', make('C'));

		await reg.flushAll('app_quit');

		// 'A:start','A:end','B:start','B:end','C:start','C:end' の順
		expect(order).toEqual(['A:start', 'A:end', 'B:start', 'B:end', 'C:start', 'C:end']);
	});

	it('個別失敗は swallow して残りを止めない (1 件の I/O 失敗で他 Note を失わせない)', async () => {
		const reg = createPendingFlushRegistry();
		const fa = vi.fn().mockResolvedValue(undefined);
		const fb = vi.fn().mockRejectedValue(new Error('boom'));
		const fc = vi.fn().mockResolvedValue(undefined);
		reg.register('a', fa);
		reg.register('b', fb);
		reg.register('c', fc);

		await reg.flushAll('app_quit');

		expect(fa).toHaveBeenCalledTimes(1);
		expect(fb).toHaveBeenCalledTimes(1);
		expect(fc).toHaveBeenCalledTimes(1);
	});

	it('unregister 後の entry は flushAll で呼ばれない', async () => {
		const reg = createPendingFlushRegistry();
		const fa = vi.fn().mockResolvedValue(undefined);
		reg.register('a', fa);
		reg.unregister('a');

		await reg.flushAll('app_quit');

		expect(fa).not.toHaveBeenCalled();
		expect(reg.size()).toBe(0);
	});

	it('同一 id を再 register すると上書きされ最新版のみ呼ばれる', async () => {
		const reg = createPendingFlushRegistry();
		const old = vi.fn().mockResolvedValue(undefined);
		const fresh = vi.fn().mockResolvedValue(undefined);
		reg.register('a', old);
		reg.register('a', fresh);

		await reg.flushAll('app_quit');

		expect(old).not.toHaveBeenCalled();
		expect(fresh).toHaveBeenCalledTimes(1);
	});

	it('flushAll 実行中に register された新 entry はその回には含まれない (snapshot semantics)', async () => {
		const reg = createPendingFlushRegistry();
		let resolveA: () => void = () => {};
		const fa = vi.fn().mockImplementation(
			() =>
				new Promise<void>((resolve) => {
					resolveA = resolve;
				})
		);
		const fb = vi.fn().mockResolvedValue(undefined);
		reg.register('a', fa);

		const p = reg.flushAll('app_quit');
		// flush 中に B が register された (例: 別 Block が EDITING で typing)
		reg.register('b', fb);
		resolveA();
		await p;

		expect(fa).toHaveBeenCalledTimes(1);
		expect(fb).not.toHaveBeenCalled();
	});
});
