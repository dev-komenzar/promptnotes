import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands';
import { markdown } from '@codemirror/lang-markdown';
import { syntaxHighlighting, defaultHighlightStyle } from '@codemirror/language';
import { EditorState, Text, type Extension } from '@codemirror/state';
import { EditorView, drawSelection, keymap } from '@codemirror/view';

const MARKDOWN_LIST_PREFIX = /^(\s*)(?:[-*+]\s|(\d+)\.\s)/;

/**
 * CommonMark backslash-escape 判定 (https://spec.commonmark.org/0.31.2/#backslash-escapes)。
 * pos の文字が直前の連続バックスラッシュによってエスケープされているかを返す。
 * `\\` は literal `\` を生むため、連続 `\` 数が奇数のときのみエスケープ扱い。
 */
export function isEscapedAt(doc: Text | string, pos: number): boolean {
	const read = (i: number): string =>
		typeof doc === 'string' ? doc.charAt(i) : doc.sliceString(i, i + 1);
	let count = 0;
	let p = pos - 1;
	while (p >= 0 && read(p) === '\\') {
		count++;
		p--;
	}
	return count % 2 === 1;
}

/**
 * Draft / Block body 共通の CodeMirror 6 構成
 * (screen-1.md#notes-codemirror-consistency / #notes-markdown-helpers)。
 * 別レンダラへの差し替えは禁止 (spec 末尾の禁止事項 #3)。
 */
export function createEditorState(options: {
	doc: string;
	onSubmit: () => boolean;
	onChange?: (next: string) => void;
	readOnly?: boolean;
	extraExtensions?: Extension[];
}): EditorState {
	const { doc, onSubmit, onChange, readOnly = false, extraExtensions = [] } = options;

	const submitBinding = keymap.of([
		{
			key: 'Mod-Enter',
			run: () => onSubmit()
		}
	]);

	const listContinuationKeymap = keymap.of([
		{
			key: 'Enter',
			run: (view) => {
				const { state } = view;
				const range = state.selection.main;
				if (!range.empty) return false;
				const line = state.doc.lineAt(range.head);
				const match = MARKDOWN_LIST_PREFIX.exec(line.text);
				if (!match) return false;
				const [matched, indent, ordinal] = match;
				if (matched.length === line.text.length) {
					// 空項目 → リスト終了
					view.dispatch({
						changes: { from: line.from, to: line.to, insert: '' },
						selection: { anchor: line.from }
					});
					return true;
				}
				const next = ordinal !== undefined ? `${indent}${Number(ordinal) + 1}. ` : matched;
				view.dispatch({
					changes: { from: range.head, insert: `\n${next}` },
					selection: { anchor: range.head + 1 + next.length },
					scrollIntoView: true
				});
				return true;
			}
		}
	]);

	const boldBracketKeymap = keymap.of([
		{
			key: '*',
			run: (view) => {
				const { state } = view;
				const range = state.selection.main;
				if (!range.empty) return false;
				const prev = range.head - 1;
				if (prev < 0) return false;
				const before = state.doc.sliceString(prev, range.head);
				if (before !== '*') return false;
				// 直前 `*` が backslash-escape されている (`\*`) なら補助を抑止 (ori-00s)
				if (isEscapedAt(state.doc, prev)) return false;
				// 直前が `*` → 4 連続 `**|**` の挿入は spec の `**` 入力時補完を実現する
				view.dispatch({
					changes: { from: range.head, insert: '*****' },
					selection: { anchor: range.head + 3 }
				});
				return true;
			}
		}
	]);

	const updateListener = onChange
		? EditorView.updateListener.of((update) => {
				if (update.docChanged) {
					onChange(update.state.doc.toString());
				}
			})
		: undefined;

	const extensions: Extension[] = [
		history(),
		drawSelection(),
		markdown(),
		syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
		submitBinding,
		listContinuationKeymap,
		boldBracketKeymap,
		keymap.of([indentWithTab, ...defaultKeymap, ...historyKeymap]),
		EditorView.lineWrapping,
		EditorState.readOnly.of(readOnly),
		...extraExtensions
	];

	if (updateListener) extensions.push(updateListener);

	return EditorState.create({ doc, extensions });
}
