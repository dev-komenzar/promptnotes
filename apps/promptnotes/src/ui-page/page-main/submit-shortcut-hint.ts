/** CodeMirror の Mod-Enter と同じ基準 (navigator.platform に Mac を含むか) で送信ショートカットのヒントを返す。 */
export function submitShortcutHint(platform: string): string {
	return /Mac/.test(platform) ? '⌘↵' : 'Ctrl+↵';
}
