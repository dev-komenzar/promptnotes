import { describe, expect, it } from 'vitest';
import { submitShortcutHint } from './submit-shortcut-hint';

describe('page:page-main submit-shortcut-hint', () => {
	it('spec#fields-draft — Mac の platform では ⌘↵ を返す', () => {
		expect(submitShortcutHint('MacIntel')).toBe('⌘↵');
	});

	it('spec#fields-draft — Linux の platform では Ctrl+↵ を返す', () => {
		expect(submitShortcutHint('Linux x86_64')).toBe('Ctrl+↵');
	});

	it('spec#fields-draft — Windows の platform では Ctrl+↵ を返す', () => {
		expect(submitShortcutHint('Win32')).toBe('Ctrl+↵');
	});
});
