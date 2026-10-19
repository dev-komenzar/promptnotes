import { afterEach, describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { EditorView } from '@codemirror/view';
import PageMain from '../PageMain.svelte';
import { draftStore } from '../stores/draft.svelte';
// dark 配色は layout.css の上書きで与えるため読み込む (component test には +layout.svelte が無い)
import '../../../routes/layout.css';

type Theme = 'Light' | 'Dark';
type Rgb = [number, number, number];

// URL / link label / 水平線 / コメント / escape / link title (defaultHighlightStyle が文字色を指定するトークン)
const HIGHLIGHT_BODY = [
	'<https://example.com>',
	'',
	'---',
	'',
	'<!-- comment -->',
	'',
	'\\* escaped',
	'',
	'[label]: /url "title"'
].join('\n');

function renderPage(theme: Theme, body = 'hello world') {
	return render(PageMain, {
		loadSettingsFn: async () => ({
			storage_dir: '/tmp',
			theme,
			sort_preference: { field: 'created_at' as const, direction: 'desc' as const }
		}),
		listNotesFn: async () => ({
			notes: [
				{
					id: '20260628_120000',
					body,
					tags: [],
					created_at: '2026-06-28 12:00:00',
					updated_at: '2026-06-28 12:00:00'
				}
			]
		})
	});
}

function editorIn(container: HTMLElement, testid: string): HTMLElement {
	const el = container.querySelector<HTMLElement>(`[data-testid="${testid}"] .cm-editor`);
	if (!el) throw new Error(`${testid} .cm-editor not found`);
	return el;
}

function viewOf(editor: HTMLElement): EditorView {
	const view = EditorView.findFromDOM(editor);
	if (!view) throw new Error('EditorView not found');
	return view;
}

// computed color (oklch 等を含む) を canvas で sRGB に正規化し、祖先の背景から順に合成する
function composite(colors: string[]): Rgb {
	const canvas = document.createElement('canvas');
	canvas.width = 1;
	canvas.height = 1;
	const ctx = canvas.getContext('2d');
	if (!ctx) throw new Error('2d context unavailable');
	ctx.fillStyle = '#ffffff';
	ctx.fillRect(0, 0, 1, 1);
	for (const c of colors) {
		ctx.fillStyle = c;
		ctx.fillRect(0, 0, 1, 1);
	}
	const [r, g, b] = ctx.getImageData(0, 0, 1, 1).data;
	return [r, g, b];
}

function backgroundsFromRoot(el: Element): string[] {
	const layers: string[] = [];
	for (let n: Element | null = el; n; n = n.parentElement) {
		layers.unshift(getComputedStyle(n).backgroundColor);
	}
	return layers;
}

function luminance([r, g, b]: Rgb): number {
	const lin = (v: number) => {
		const s = v / 255;
		return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
	};
	return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
}

function contrast(a: Rgb, b: Rgb): number {
	const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
	return (hi + 0.05) / (lo + 0.05);
}

function selectionBackground(editor: HTMLElement): HTMLElement | null {
	return editor.querySelector<HTMLElement>('.cm-selectionBackground');
}

async function selectAll(editor: HTMLElement, focus: boolean): Promise<HTMLElement> {
	const view = viewOf(editor);
	view.dispatch({ selection: { anchor: 0, head: view.state.doc.length } });
	if (focus) {
		view.focus();
		await expect.poll(() => editor.classList.contains('cm-focused')).toBe(true);
	} else {
		view.contentDOM.blur();
		await expect.poll(() => editor.classList.contains('cm-focused')).toBe(false);
	}
	await expect.poll(() => selectionBackground(editor)).not.toBeNull();
	return selectionBackground(editor) as HTMLElement;
}

function selectionContrast(editor: HTMLElement, selBg: HTMLElement): number {
	const bg = composite([...backgroundsFromRoot(editor), getComputedStyle(selBg).backgroundColor]);
	const text = composite([getComputedStyle(viewOf(editor).contentDOM).color]);
	return contrast(text, bg);
}

async function waitDark(dark: boolean) {
	await expect.poll(() => document.documentElement.classList.contains('dark')).toBe(dark);
}

async function blockEditor(container: HTMLElement): Promise<HTMLElement> {
	await expect
		.poll(() => container.querySelector('[data-testid="page.page-main.block-body"] .cm-editor'))
		.not.toBeNull();
	return editorIn(container, 'page.page-main.block-body');
}

afterEach(() => {
	draftStore.setBody('');
	document.documentElement.classList.remove('dark');
});

describe('page:page-main dark theme CodeMirror contrast', () => {
	describe('spec#tp-theme-codemirror-contrast — 選択範囲 (I-PM16b)', () => {
		it('TP-T12 — dark で Draft の選択背景と本文文字色のコントラスト比が 4.5:1 以上 (focused)', async () => {
			const { container } = renderPage('Dark');
			await waitDark(true);
			draftStore.setBody('hello world');
			const editor = editorIn(container, 'page.page-main.draft-body');
			await expect.poll(() => viewOf(editor).state.doc.toString()).toBe('hello world');

			const selBg = await selectAll(editor, true);
			expect(selectionContrast(editor, selBg)).toBeGreaterThanOrEqual(4.5);
		});

		it('TP-T12 — dark で Draft の選択背景と本文文字色のコントラスト比が 4.5:1 以上 (unfocused)', async () => {
			const { container } = renderPage('Dark');
			await waitDark(true);
			draftStore.setBody('hello world');
			const editor = editorIn(container, 'page.page-main.draft-body');
			await expect.poll(() => viewOf(editor).state.doc.toString()).toBe('hello world');

			const selBg = await selectAll(editor, false);
			expect(selectionContrast(editor, selBg)).toBeGreaterThanOrEqual(4.5);
		});

		it('TP-T13 — dark で Block body の選択背景と本文文字色のコントラスト比が 4.5:1 以上 (focused / unfocused)', async () => {
			const { container } = renderPage('Dark');
			await waitDark(true);
			const editor = await blockEditor(container);

			const focusedBg = await selectAll(editor, true);
			expect(selectionContrast(editor, focusedBg)).toBeGreaterThanOrEqual(4.5);

			const unfocusedBg = await selectAll(editor, false);
			expect(selectionContrast(editor, unfocusedBg)).toBeGreaterThanOrEqual(4.5);
		});

		it('TP-T14 — light では CodeMirror 既定の選択背景色のまま', async () => {
			const { container } = renderPage('Light');
			await waitDark(false);
			const editor = await blockEditor(container);

			// CodeMirror baseTheme 既定: focused #d7d4f0 / unfocused #d9d9d9
			const focusedBg = await selectAll(editor, true);
			expect(composite([getComputedStyle(focusedBg).backgroundColor])).toEqual([215, 212, 240]);

			const unfocusedBg = await selectAll(editor, false);
			expect(composite([getComputedStyle(unfocusedBg).backgroundColor])).toEqual([217, 217, 217]);
		});
	});

	describe('spec#tp-theme-codemirror-contrast — シンタックスハイライト (I-PM16c)', () => {
		function coloredTokens(editor: HTMLElement): HTMLElement[] {
			const base = getComputedStyle(viewOf(editor).contentDOM).color;
			return Array.from(editor.querySelectorAll<HTMLElement>('.cm-line span')).filter(
				(s) => getComputedStyle(s).color !== base
			);
		}

		function tokenContrasts(editor: HTMLElement): number[] {
			const bg = composite(backgroundsFromRoot(editor));
			return coloredTokens(editor).map((s) => contrast(composite([getComputedStyle(s).color]), bg));
		}

		it('TP-T15 — dark で Draft / Block body の色付きトークンと背景のコントラスト比が 4.5:1 以上', async () => {
			const { container } = renderPage('Dark', HIGHLIGHT_BODY);
			await waitDark(true);
			draftStore.setBody(HIGHLIGHT_BODY);
			const draft = editorIn(container, 'page.page-main.draft-body');
			const block = await blockEditor(container);

			for (const editor of [draft, block]) {
				// URL / label / 水平線 / コメント / escape / title の各トークンが描画済みであること
				await expect.poll(() => coloredTokens(editor).length).toBeGreaterThanOrEqual(6);
				for (const ratio of tokenContrasts(editor)) {
					expect(ratio).toBeGreaterThanOrEqual(4.5);
				}
			}
		});

		it('TP-T15 — light では既定ハイライト配色 (defaultHighlightStyle) のまま', async () => {
			const { container } = renderPage('Light', '<https://example.com>');
			await waitDark(false);
			const editor = await blockEditor(container);

			await expect.poll(() => coloredTokens(editor).length).toBeGreaterThan(0);
			// defaultHighlightStyle の url 色 #219
			const url = Array.from(editor.querySelectorAll<HTMLElement>('.cm-line span')).find(
				(s) => s.textContent === 'https://example.com'
			);
			expect(url).toBeTruthy();
			expect(composite([getComputedStyle(url as HTMLElement).color])).toEqual([34, 17, 153]);
		});
	});
});
