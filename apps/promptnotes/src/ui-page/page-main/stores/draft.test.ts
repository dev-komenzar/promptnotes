import { describe, expect, it, vi } from 'vitest';
import type { CreateNoteOutcome } from '$lib/note-capture/slices/create-note';
import { createDraftStore } from './draft.svelte';

function created(id = 'note-1', createdAt = '2026-06-26T00:00:00Z'): CreateNoteOutcome {
	return { outcome: 'created', id, created_at: createdAt };
}

describe('page:page-main draft store', () => {
	it('spec#tp-golden-create — submit が created を返すと body と tags をクリアする', async () => {
		const create = vi.fn().mockResolvedValue(created('note-1'));
		const store = createDraftStore({ create });

		store.setBody('hello');
		store.addTag('work');
		store.addTag('ai');
		const outcome = await store.submit();

		expect(create).toHaveBeenCalledWith('hello', ['work', 'ai']);
		expect(outcome).toStrictEqual({
			outcome: 'created',
			id: 'note-1',
			created_at: '2026-06-26T00:00:00Z'
		});
		expect(store.body).toBe('');
		expect(store.tags).toStrictEqual([]);
	});

	it('submit に tags が渡される', async () => {
		const create = vi.fn().mockResolvedValue(created('note-1'));
		const store = createDraftStore({ create });

		store.setBody('draft');
		store.addTag('urgent');
		await store.submit();

		expect(create).toHaveBeenCalledWith('draft', ['urgent']);
	});

	it('addTag は重複を追加しない', () => {
		const store = createDraftStore({ create: vi.fn() });

		store.addTag('work');
		store.addTag('work');
		store.addTag('Work'); // case-insensitive due to lowercase

		expect(store.tags).toStrictEqual(['work']);
	});

	it('addTag は不正な文字を含むタグを reject する', () => {
		const store = createDraftStore({ create: vi.fn() });

		expect(store.addTag('bad tag').outcome).toBe('invalid');
		expect(store.addTag('bad,tag').outcome).toBe('invalid');
		expect(store.addTag('[bracket]').outcome).toBe('invalid');

		expect(store.tags).toStrictEqual([]);
	});

	it('addTag は空文字列を reject する', () => {
		const store = createDraftStore({ create: vi.fn() });

		expect(store.addTag('').outcome).toBe('invalid');
		expect(store.addTag('   ').outcome).toBe('invalid');

		expect(store.tags).toStrictEqual([]);
	});

	it('addTag は空白を trim し lowercase で正規化する', () => {
		const store = createDraftStore({ create: vi.fn() });

		store.addTag('  Work  ');
		expect(store.tags).toStrictEqual(['work']);
	});

	it('removeTag は指定タグを削除する', () => {
		const store = createDraftStore({ create: vi.fn() });

		store.addTag('work');
		store.addTag('ai');
		store.removeTag('work');

		expect(store.tags).toStrictEqual(['ai']);
	});

	it('removeTag は存在しないタグでもエラーにならない', () => {
		const store = createDraftStore({ create: vi.fn() });

		store.addTag('work');
		store.removeTag('nonexistent');

		expect(store.tags).toStrictEqual(['work']);
	});

	it('clear() は body と tags の両方をクリアする', () => {
		const store = createDraftStore({ create: vi.fn() });

		store.setBody('draft');
		store.addTag('work');
		store.clear();

		expect(store.body).toBe('');
		expect(store.tags).toStrictEqual([]);
	});

	it('spec#tp-empty-body-noop — outcome が no_op の場合 body は変えない', async () => {
		const create = vi.fn().mockResolvedValue({ outcome: 'no_op' } satisfies CreateNoteOutcome);
		const store = createDraftStore({ create });

		const outcome = await store.submit();

		expect(create).toHaveBeenCalledWith('', []);
		expect(outcome).toStrictEqual({ outcome: 'no_op' });
		expect(store.body).toBe('');
	});

	it('spec#tp-empty-body-noop — whitespace-only body でも create-note slice に委譲する (C-CN3 は Rust 側で判定)', async () => {
		const create = vi.fn().mockResolvedValue({ outcome: 'no_op' } satisfies CreateNoteOutcome);
		const store = createDraftStore({ create });

		store.setBody('   \n  ');
		await store.submit();

		expect(create).toHaveBeenCalledWith('   \n  ', []);
		expect(store.body).toBe('   \n  ');
	});

	it('spec#impl-notes(C-CN6) — submit 中に再 submit しても重複呼出しない', async () => {
		let resolve: (value: CreateNoteOutcome) => void = () => {};
		const create = vi.fn().mockImplementation(
			() =>
				new Promise<CreateNoteOutcome>((r) => {
					resolve = r;
				})
		);
		const store = createDraftStore({ create });

		store.setBody('hi');
		const first = store.submit();
		const second = store.submit();

		expect(create).toHaveBeenCalledTimes(1);
		resolve(created('note-1'));

		await first;
		const secondOutcome = await second;
		expect(secondOutcome).toStrictEqual({ outcome: 'no_op' });
	});
});
