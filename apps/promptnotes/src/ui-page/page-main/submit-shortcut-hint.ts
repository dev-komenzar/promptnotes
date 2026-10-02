/** CodeMirror の Mod-Enter と同じ基準 (userAgent に Mac を含むか) で送信ショートカットのヒントを返す。 */
export function submitShortcutHint(userAgent: string): string {
	return /Mac/.test(userAgent) ? '⌘↵' : 'Ctrl+↵';
}
