import { describe, expect, it } from 'vitest';
import { Text } from '@codemirror/state';
import { isEscapedAt } from './setup';

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
