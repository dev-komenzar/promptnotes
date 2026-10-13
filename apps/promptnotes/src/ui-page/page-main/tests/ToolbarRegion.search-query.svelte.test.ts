import { beforeEach, describe, expect, it, vi } from 'vitest';
import { page } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';

// adapter 境界 (update-feed-filter slice の Tauri 呼出) のみ fake 化する。
// backend 同様 NFKC + lowercase + trim して遅延応答する (I-F1 / C-UF2)。
const updateFeedFilter = vi.hoisted(() =>
	vi.fn(
		(input: { kind: string; raw?: string }) =>
			new Promise((resolve) => {
				setTimeout(() => {
					const n = (input.raw ?? '').normalize('NFKC').toLowerCase().trim();
					resolve({ query: n === '' ? null : n, date_range: { kind: 'all' }, tag: null });
				}, 30);
			})
	)
);
vi.mock('$lib/note-feed/slices/update-feed-filter', () => ({ updateFeedFilter }));

const { default: ToolbarRegion } = await import('../regions/ToolbarRegion.svelte');

const wait = (ms = 0) => new Promise((r) => setTimeout(r, ms));

function searchInput(): HTMLInputElement {
	return page.getByRole('searchbox', { name: 'Search' }).element() as HTMLInputElement;
}

function input(el: HTMLInputElement, value: string, isComposing: boolean) {
	el.value = value;
	el.dispatchEvent(new InputEvent('input', { bubbles: true, isComposing }));
}

function composition(el: HTMLInputElement, type: 'compositionstart' | 'compositionend') {
	el.dispatchEvent(new CompositionEvent(type, { bubbles: true, data: el.value }));
}

function setQueryRaws(): string[] {
	return updateFeedFilter.mock.calls
		.map(([arg]) => arg)
		.filter((arg) => arg.kind === 'set_query')
		.map((arg) => arg.raw as string);
}

beforeEach(() => {
	updateFeedFilter.mockClear();
});

describe('page:page-main spec#tp-search-query-input (bug:ori-fdhh)', () => {
	it('spec#invariants-cross-region I-PM11 — ASCII 入力は 1 文字ごとに即時 set_query を送る', async () => {
		render(ToolbarRegion, {});
		const el = searchInput();

		input(el, 'g', false);
		input(el, 'gp', false);
		await wait(80);

		expect(setQueryRaws()).toEqual(['g', 'gp']);
	});

	it('spec#invariants-cross-region I-PM20 — IME 変換中は set_query を送らず、入力欄の値も書き換えない', async () => {
		render(ToolbarRegion, {});
		const el = searchInput();
		el.focus();

		composition(el, 'compositionstart');
		const observed: string[] = [];
		for (const v of ['k', 'か', 'かn', 'かな']) {
			input(el, v, true);
			await wait();
			observed.push(el.value);
		}
		await wait(80);

		expect(observed).toEqual(['k', 'か', 'かn', 'かな']);
		expect(el.value).toBe('かな');
		expect(setQueryRaws()).toEqual([]);
	});

	it('spec#invariants-cross-region I-PM20 — 確定時に set_query を 1 回送る (input → compositionend の順: Chromium)', async () => {
		render(ToolbarRegion, {});
		const el = searchInput();

		composition(el, 'compositionstart');
		input(el, 'k', true);
		input(el, 'かな', true);
		composition(el, 'compositionend');
		await wait(80);

		expect(setQueryRaws()).toEqual(['かな']);
		expect(el.value).toBe('かな');
	});

	it('spec#invariants-cross-region I-PM20 — 確定時に set_query を 1 回送る (compositionend → input の順: WebKit)', async () => {
		render(ToolbarRegion, {});
		const el = searchInput();

		composition(el, 'compositionstart');
		input(el, 'k', true);
		el.value = 'かな';
		composition(el, 'compositionend');
		input(el, 'かな', false);
		await wait(80);

		expect(setQueryRaws()).toEqual(['かな']);
		expect(el.value).toBe('かな');
	});

	it('spec#invariants-cross-region I-PM20 — 入力欄は raw 入力を保ち、正規化値 (lowercase / trim) で上書きしない', async () => {
		render(ToolbarRegion, {});
		const el = searchInput();

		input(el, 'Foo', false);
		input(el, 'Foo ', false);
		await wait(80);

		expect(el.value).toBe('Foo ');
		expect(setQueryRaws()).toEqual(['Foo', 'Foo ']);
	});

	it('spec#invariants-cross-region I-PM20 — Esc で composition をキャンセルすると変換前の値に戻り、追加送信しない', async () => {
		render(ToolbarRegion, {});
		const el = searchInput();

		input(el, 'abc', false);
		composition(el, 'compositionstart');
		input(el, 'abcか', true);
		// Esc: 未確定文字が破棄され、変換前の値で composition が終わる
		input(el, 'abc', true);
		composition(el, 'compositionend');
		await wait(80);

		expect(el.value).toBe('abc');
		expect(setQueryRaws()).toEqual(['abc']);
	});

	it('spec#invariants-cross-region I-PM20 — 確定値が直前の送信値と同じなら再送しない', async () => {
		render(ToolbarRegion, {});
		const el = searchInput();

		composition(el, 'compositionstart');
		input(el, 'かな', true);
		composition(el, 'compositionend');
		// 同じ確定値で再度 composition を確定 (WebKit 順の確定 input も含む)
		composition(el, 'compositionstart');
		input(el, 'かな', true);
		composition(el, 'compositionend');
		input(el, 'かな', false);
		await wait(80);

		expect(el.value).toBe('かな');
		expect(setQueryRaws()).toEqual(['かな']);
	});

	it('spec#tp-search-query-input — Clear all で入力欄が空になり clear_all を送る', async () => {
		render(ToolbarRegion, {});
		const el = searchInput();

		input(el, 'abc', false);
		await wait(80);
		await page.getByRole('button', { name: 'Clear all filters' }).click();
		await wait(80);

		expect(el.value).toBe('');
		expect(updateFeedFilter.mock.calls.at(-1)?.[0]).toEqual({ kind: 'clear_all' });
	});
});
