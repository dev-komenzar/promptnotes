import { afterEach, describe, expect, it } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import PageMain from '../PageMain.svelte';
import { draftStore } from '../stores/draft.svelte';
// 実寸レイアウトを検証するため Tailwind を読み込む (component test には +layout.svelte が無い)
import '../../../routes/layout.css';

const noopLoadSettings = async () => ({
	storage_dir: '/tmp',
	theme: 'System' as const,
	sort_preference: { field: 'created_at' as const, direction: 'desc' as const }
});

const noopListNotes = async () => ({ notes: [] });

const LONG_BODY = Array.from({ length: 200 }, (_, i) => `line ${i + 1}`).join('\n');

function renderPage() {
	return render(PageMain, { loadSettingsFn: noopLoadSettings, listNotesFn: noopListNotes });
}

function draftBody(container: HTMLElement): HTMLElement {
	const el = container.querySelector<HTMLElement>('[data-testid="screen-1-draft-body"]');
	if (!el) throw new Error('screen-1-draft-body not found');
	return el;
}

function scroller(container: HTMLElement): HTMLElement {
	const el = draftBody(container).querySelector<HTMLElement>('.cm-scroller');
	if (!el) throw new Error('.cm-scroller not found');
	return el;
}

function isInViewport(el: Element): boolean {
	const rect = el.getBoundingClientRect();
	return rect.top >= 0 && rect.bottom <= window.innerHeight;
}

afterEach(() => {
	draftStore.setBody('');
});

describe('page:page-main draft max height', () => {
	it('spec#tp-draft-max-height — 長文の本文エディタは 70vh 以下に収まりエディタ内スクロールできる', async () => {
		await page.viewport(1024, 800);
		const { container } = renderPage();
		draftStore.setBody(LONG_BODY);

		await expect.poll(() => scroller(container).scrollHeight).toBeGreaterThan(800);

		expect(draftBody(container).getBoundingClientRect().height).toBeLessThanOrEqual(
			window.innerHeight * 0.7
		);
		const sc = scroller(container);
		expect(sc.scrollHeight).toBeGreaterThan(sc.clientHeight);
		// Draft region やページ全体はスクロールしない
		expect(document.documentElement.scrollHeight).toBeLessThanOrEqual(window.innerHeight);
	});

	it('spec#tp-draft-max-height — 短い本文では最大高さまで伸びない', async () => {
		await page.viewport(1024, 800);
		const { container } = renderPage();
		draftStore.setBody('hello');

		await expect.poll(() => scroller(container).textContent).toContain('hello');

		// 1 行ぶん + padding 程度 (min-h 3rem を大きく超えない)
		expect(draftBody(container).getBoundingClientRect().height).toBeLessThan(96);
	});

	it('spec#tp-draft-max-height — 長文末尾のカーソル行がエディタの可視範囲内にある', async () => {
		await page.viewport(1024, 800);
		const { container } = renderPage();
		draftStore.setBody(LONG_BODY);
		await expect.poll(() => scroller(container).scrollHeight).toBeGreaterThan(800);

		const content = draftBody(container).querySelector<HTMLElement>('.cm-content');
		content?.focus();
		await userEvent.keyboard('{Control>}{End}{/Control}');

		await expect
			.poll(() => {
				const sel = window.getSelection();
				if (!sel || sel.rangeCount === 0) return false;
				const caret = sel.getRangeAt(0).getBoundingClientRect();
				const sc = scroller(container);
				const box = sc.getBoundingClientRect();
				return (
					sc.scrollTop > 0 &&
					caret.top >= box.top &&
					caret.bottom <= box.bottom &&
					caret.top >= 0 &&
					caret.bottom <= window.innerHeight
				);
			})
			.toBe(true);
	});

	it('spec#tp-draft-max-height — 低い viewport でも + Add とタグ入力は viewport 内に表示される', async () => {
		await page.viewport(1024, 300);
		const { container } = renderPage();
		draftStore.setBody(LONG_BODY);
		await expect.poll(() => scroller(container).scrollHeight).toBeGreaterThan(300);

		const submit = container.querySelector('[data-testid="screen-1-draft-submit"]');
		const tagInput = container.querySelector('[data-testid="screen-1-draft-tag-input"]');
		expect(submit && isInViewport(submit)).toBe(true);
		expect(tagInput && isInViewport(tagInput)).toBe(true);
	});
});
