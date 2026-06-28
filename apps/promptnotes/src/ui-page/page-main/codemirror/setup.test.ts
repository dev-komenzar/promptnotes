// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import { Text } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import { createEditorState, isEscapedAt } from './setup';

describe('feature:page-main codemirror — isEscapedAt (string input, ori-00s)', () => {
	it('no preceding backslash → false', () => {
		expect(isEscapedAt('abc', 0)).toBe(false);
		expect(isEscapedAt('abc', 1)).toBe(false);
		expect(isEscapedAt('abc', 2)).toBe(false);
	});

	it('CommonMark §6.1 — `\\*` の `*` (pos 1) は escape', () => {
		expect(isEscapedAt('\\*', 1)).toBe(true);
	});

	it('CommonMark §6.1 — `\\_` の `_` も同じく escape', () => {
		expect(isEscapedAt('\\_', 1)).toBe(true);
	});

	it('CommonMark §6.1 — `\\[` / `\\]` も escape', () => {
		expect(isEscapedAt('\\[', 1)).toBe(true);
		expect(isEscapedAt('\\]', 1)).toBe(true);
	});

	it('CommonMark §6.1 — `\\\\` (literal `\\`) の 2 番目 `\\` は escape (preceded by 1 backslash)', () => {
		expect(isEscapedAt('\\\\', 1)).toBe(true);
	});

	it('連続 backslash の偶奇 — `\\\\*` (literal `\\` + raw `*`) の `*` は escape ではない', () => {
		expect(isEscapedAt('\\\\*', 2)).toBe(false);
	});

	it('連続 backslash の偶奇 — `\\\\\\*` (literal `\\` + escaped `*`) の `*` は escape', () => {
		expect(isEscapedAt('\\\\\\*', 3)).toBe(true);
	});

	it('連続 backslash の偶奇 — `\\\\\\\\*` の `*` は escape ではない', () => {
		expect(isEscapedAt('\\\\\\\\*', 4)).toBe(false);
	});

	it('離れた backslash は無関係 — `a\\b*` の `*` は escape ではない', () => {
		expect(isEscapedAt('a\\b*', 3)).toBe(false);
	});

	it('行頭 `*` (pos 0) は backslash も無いので escape ではない', () => {
		expect(isEscapedAt('*foo', 0)).toBe(false);
	});
});

describe('feature:page-main codemirror — Block の active line ハイライト無効化 (ori-67c)', () => {
	// screen-1.md#notes-block-vertical は Block 単位の背景色 (IDLE/FOCUSED/EDITING) のみ規定。
	// 行ハイライトは spec に無いため highlightActiveLine extension は付けない。
	it('readOnly Block で初期 selection (pos 0) の line に .cm-activeLine が付かない', () => {
		const host = document.createElement('div');
		document.body.appendChild(host);
		try {
			const state = createEditorState({
				doc: 'line0\nline1\nline2',
				onSubmit: () => false,
				readOnly: true
			});
			const view = new EditorView({ state, parent: host });
			try {
				expect(host.querySelector('.cm-activeLine')).toBeNull();
			} finally {
				view.destroy();
			}
		} finally {
			host.remove();
		}
	});

	it('編集可能 Block でも未フォーカス時に .cm-activeLine が付かない', () => {
		const host = document.createElement('div');
		document.body.appendChild(host);
		try {
			const state = createEditorState({
				doc: 'line0\nline1',
				onSubmit: () => false,
				readOnly: false
			});
			const view = new EditorView({ state, parent: host });
			try {
				expect(host.querySelector('.cm-activeLine')).toBeNull();
			} finally {
				view.destroy();
			}
		} finally {
			host.remove();
		}
	});
});

describe('feature:page-main codemirror — isEscapedAt (Text input, ori-00s)', () => {
	it('CodeMirror Text 上でも同じ判定が成り立つ', () => {
		const doc = Text.of(['\\*']);
		expect(isEscapedAt(doc, 1)).toBe(true);
	});

	it('multi-line Text でも consecutive backslash count が機能する', () => {
		const doc = Text.of(['abc', '\\\\*']);
		// `abc\n\\\\*` → `*` は pos 6、その直前 2 つは `\\` (literal `\`) → escape ではない
		expect(isEscapedAt(doc, 6)).toBe(false);
	});
});
