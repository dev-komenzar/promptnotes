import { describe, expect, it } from 'vitest';
import { submitShortcutHint } from './submit-shortcut-hint';

describe('page:page-main submit-shortcut-hint', () => {
	it('spec#fields-draft — Mac 系 UA では ⌘↵ を返す', () => {
		expect(
			submitShortcutHint(
				'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko)'
			)
		).toBe('⌘↵');
	});

	it('spec#fields-draft — Linux UA では Ctrl+↵ を返す', () => {
		expect(submitShortcutHint('Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36')).toBe('Ctrl+↵');
	});

	it('spec#fields-draft — Windows UA では Ctrl+↵ を返す', () => {
		expect(submitShortcutHint('Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36')).toBe(
			'Ctrl+↵'
		);
	});
});
