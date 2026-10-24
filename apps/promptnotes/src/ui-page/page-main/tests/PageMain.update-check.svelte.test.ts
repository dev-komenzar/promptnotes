import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { page } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import PageMain from '../PageMain.svelte';

// Tauri IPC 境界 (adapter) のみ fake 化する。page-main → check-for-updates slice wrapper →
// invoke('check_for_updates') と widget-update-toast → listen('new_version_detected') は
// production の配線のまま通す (ori-m2ra.1 の回帰防止: 起動時 check の配線漏れを検出する)。
type Handler = (event: { payload: unknown }) => void;

const tauri = vi.hoisted(() => {
	const handlers = new Map<string, Handler>();
	const calls: string[] = [];
	return {
		handlers,
		calls,
		checkResponder: { current: (() => {}) as () => void },
		listen: async (name: string, handler: Handler) => {
			handlers.set(name, handler);
			calls.push(`listen:${name}`);
			return () => handlers.delete(name);
		},
		invoke: async (cmd: string) => {
			calls.push(`invoke:${cmd}`);
			if (cmd === 'check_for_updates') {
				tauri.checkResponder.current();
				return { current_version: '0.2.2', latest_release: null };
			}
			return undefined;
		}
	};
});

vi.mock('@tauri-apps/api/event', async (importOriginal) => ({
	...(await importOriginal<typeof import('@tauri-apps/api/event')>()),
	listen: tauri.listen
}));
vi.mock('@tauri-apps/api/core', async (importOriginal) => ({
	...(await importOriginal<typeof import('@tauri-apps/api/core')>()),
	invoke: tauri.invoke
}));

const loadSettingsFn = async () => ({
	storage_dir: '/tmp',
	theme: 'System' as const,
	sort_preference: { field: 'created_at' as const, direction: 'desc' as const }
});
const listNotesFn = async () => ({ notes: [] });

const payload = {
	current_version: '0.2.2',
	latest_version: '0.2.3',
	release_url: 'https://github.com/dev-komenzar/promptnotes/releases/tag/v0.2.3',
	release_notes: 'bug fixes'
};

beforeEach(() => {
	tauri.handlers.clear();
	tauri.calls.length = 0;
	tauri.checkResponder.current = () => {};
});

afterEach(() => {
	document.documentElement.classList.remove('dark');
});

describe('page:page-main startup update check (I-U3 / I-PM3)', () => {
	it('spec#invariants I-PM3 — mount 時に check_for_updates を 1 回だけ、new_version_detected の listen 確立後に invoke する', async () => {
		render(PageMain, { loadSettingsFn, listNotesFn, quitWindow: null });

		await expect
			.poll(() => tauri.calls.filter((c) => c === 'invoke:check_for_updates').length)
			.toBe(1);
		const listenAt = tauri.calls.indexOf('listen:new_version_detected');
		const invokeAt = tauri.calls.indexOf('invoke:check_for_updates');
		expect(listenAt).toBeGreaterThanOrEqual(0);
		expect(listenAt).toBeLessThan(invokeAt);
	});

	it('screen-3 — 起動時 check が emit した NewVersionDetected で update toast が表示される', async () => {
		// Rust 側 check_for_updates は新版検出時に invoke 応答前に event を emit する。
		tauri.checkResponder.current = () => {
			tauri.handlers.get('new_version_detected')?.({ payload });
		};

		render(PageMain, { loadSettingsFn, listNotesFn, quitWindow: null });

		await expect.element(page.getByTestId('widget.widget-update-toast.root')).toBeInTheDocument();
		await expect
			.element(page.getByTestId('widget.widget-update-toast.latest-version'))
			.toHaveTextContent('0.2.3');
	});

	it('S14 — check_for_updates が reject しても toast は出ず page-main は描画を続ける', async () => {
		tauri.checkResponder.current = () => {
			throw new Error('network down');
		};

		render(PageMain, { loadSettingsFn, listNotesFn, quitWindow: null });

		await expect.poll(() => tauri.calls.includes('invoke:check_for_updates')).toBe(true);
		await expect.element(page.getByTestId('page.page-main.region-feed')).toBeInTheDocument();
		expect(document.querySelector('[data-testid="widget.widget-update-toast.root"]')).toBeNull();
	});
});
