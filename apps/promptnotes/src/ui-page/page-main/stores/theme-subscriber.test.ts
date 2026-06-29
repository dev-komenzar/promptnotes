import { describe, expect, it, vi } from 'vitest';
import {
	createThemeSubscriber,
	type MatchMediaFn,
	type ThemeChangedPayload,
	type ThemeListener
} from './theme-subscriber.svelte';

function makePayload(overrides: Partial<ThemeChangedPayload> = {}): ThemeChangedPayload {
	return { new_theme: 'Dark', ...overrides };
}

type ListenCapture = {
	emit: (payload: ThemeChangedPayload) => void;
	unsubscribe: ReturnType<typeof vi.fn>;
	listenFn: ThemeListener;
};

function makeListenCapture(): ListenCapture {
	let stored: ((payload: ThemeChangedPayload) => void) | null = null;
	const unsubscribe = vi.fn();
	const listenFn: ThemeListener = async (handler) => {
		stored = handler;
		return unsubscribe;
	};
	return {
		emit: (payload) => {
			if (!stored) throw new Error('listenFn was not awaited before emit');
			stored(payload);
		},
		unsubscribe,
		listenFn
	};
}

class FakeMediaQueryList {
	matches: boolean;
	private listeners = new Set<EventListener>();
	constructor(matches: boolean) {
		this.matches = matches;
	}
	addEventListener(_type: string, listener: EventListener): void {
		this.listeners.add(listener);
	}
	removeEventListener(_type: string, listener: EventListener): void {
		this.listeners.delete(listener);
	}
	dispatch(): void {
		for (const l of this.listeners) l({} as Event);
	}
}

function makeMatchMediaFn(initialDark: boolean): { fn: MatchMediaFn; mql: FakeMediaQueryList } {
	const mql = new FakeMediaQueryList(initialDark);
	const fn: MatchMediaFn = () => mql as unknown as MediaQueryList;
	return { fn, mql };
}

class FakeElement {
	classList = new FakeClassList();
}

class FakeClassList {
	private classes = new Set<string>();
	add(c: string): void {
		this.classes.add(c);
	}
	remove(c: string): void {
		this.classes.delete(c);
	}
	toggle(c: string, force?: boolean): void {
		if (force === undefined) {
			if (this.classes.has(c)) this.classes.delete(c);
			else this.classes.add(c);
		} else if (force) {
			this.classes.add(c);
		} else {
			this.classes.delete(c);
		}
	}
	contains(c: string): boolean {
		return this.classes.has(c);
	}
}

function makeElement(): HTMLElement {
	return new FakeElement() as unknown as HTMLElement;
}

describe('page:page-main theme-subscriber', () => {
	it('spec.md#tp-theme-apply: TP-T1 — Dark を set すると .dark class が付く', () => {
		const { fn } = makeMatchMediaFn(false);
		const el = makeElement();
		const subscriber = createThemeSubscriber({ matchMediaFn: fn, documentElement: el });

		subscriber.setTheme('Dark');

		expect(el.classList.contains('dark')).toBe(true);
	});

	it('spec.md#tp-theme-apply: TP-T2 — Light を set すると .dark class が外れる', () => {
		const { fn } = makeMatchMediaFn(true);
		const el = makeElement();
		el.classList.add('dark');
		const subscriber = createThemeSubscriber({ matchMediaFn: fn, documentElement: el });

		subscriber.setTheme('Light');

		expect(el.classList.contains('dark')).toBe(false);
	});

	it('spec.md#tp-theme-apply: TP-T3 — System + media dark=true で .dark が付く', () => {
		const { fn } = makeMatchMediaFn(true);
		const el = makeElement();
		const subscriber = createThemeSubscriber({ matchMediaFn: fn, documentElement: el });

		subscriber.setTheme('System');

		expect(el.classList.contains('dark')).toBe(true);
	});

	it('spec.md#tp-theme-apply: TP-T4 — System + media dark=false で .dark が外れる', () => {
		const { fn } = makeMatchMediaFn(false);
		const el = makeElement();
		el.classList.add('dark');
		const subscriber = createThemeSubscriber({ matchMediaFn: fn, documentElement: el });

		subscriber.setTheme('System');

		expect(el.classList.contains('dark')).toBe(false);
	});

	it('spec.md#tp-theme-system-media: TP-T5 — System 中 media query dark→light 変化で反映', () => {
		const { fn, mql } = makeMatchMediaFn(true);
		const el = makeElement();
		const subscriber = createThemeSubscriber({ matchMediaFn: fn, documentElement: el });

		subscriber.setTheme('System');
		expect(el.classList.contains('dark')).toBe(true);

		mql.matches = false;
		mql.dispatch();

		expect(el.classList.contains('dark')).toBe(false);
	});

	it('spec.md#tp-theme-system-media: TP-T6 — System 中 media query light→dark 変化で反映', () => {
		const { fn, mql } = makeMatchMediaFn(false);
		const el = makeElement();
		const subscriber = createThemeSubscriber({ matchMediaFn: fn, documentElement: el });

		subscriber.setTheme('System');
		expect(el.classList.contains('dark')).toBe(false);

		mql.matches = true;
		mql.dispatch();

		expect(el.classList.contains('dark')).toBe(true);
	});

	it('spec.md#tp-theme-system-media: TP-T7 — Dark 固定中は media query 変化を無視', () => {
		const { fn, mql } = makeMatchMediaFn(false);
		const el = makeElement();
		const subscriber = createThemeSubscriber({ matchMediaFn: fn, documentElement: el });

		subscriber.setTheme('Dark');
		expect(el.classList.contains('dark')).toBe(true);

		mql.matches = false;
		mql.dispatch();

		expect(el.classList.contains('dark')).toBe(true);
	});

	it('spec.md#tp-theme-system-media: TP-T7 — Light 固定中は media query 変化を無視', () => {
		const { fn, mql } = makeMatchMediaFn(true);
		const el = makeElement();
		const subscriber = createThemeSubscriber({ matchMediaFn: fn, documentElement: el });

		subscriber.setTheme('Light');
		expect(el.classList.contains('dark')).toBe(false);

		mql.matches = true;
		mql.dispatch();

		expect(el.classList.contains('dark')).toBe(false);
	});

	it('spec.md#tp-theme-system-media: TP-T7 — System→Dark 切替で media listener が remove される', () => {
		const { fn, mql } = makeMatchMediaFn(false);
		const removeSpy = vi.spyOn(mql, 'removeEventListener');
		const el = makeElement();
		const subscriber = createThemeSubscriber({ matchMediaFn: fn, documentElement: el });

		subscriber.setTheme('System');
		subscriber.setTheme('Dark');

		expect(removeSpy).toHaveBeenCalledTimes(1);
		mql.matches = false;
		mql.dispatch();
		expect(el.classList.contains('dark')).toBe(true);
	});

	it('spec.md#tp-theme-changed-event: TP-T8 — theme_changed event で Dark → .dark 付与', async () => {
		const cap = makeListenCapture();
		const { fn } = makeMatchMediaFn(false);
		const el = makeElement();
		const subscriber = createThemeSubscriber({
			listenFn: cap.listenFn,
			matchMediaFn: fn,
			documentElement: el
		});

		await subscriber.start();
		cap.emit(makePayload({ new_theme: 'Dark' }));

		expect(el.classList.contains('dark')).toBe(true);
	});

	it('spec.md#tp-theme-changed-event: TP-T9 — theme_changed event で Light → .dark 削除', async () => {
		const cap = makeListenCapture();
		const { fn } = makeMatchMediaFn(true);
		const el = makeElement();
		el.classList.add('dark');
		const subscriber = createThemeSubscriber({
			listenFn: cap.listenFn,
			matchMediaFn: fn,
			documentElement: el
		});

		await subscriber.start();
		cap.emit(makePayload({ new_theme: 'Light' }));

		expect(el.classList.contains('dark')).toBe(false);
	});

	it('spec.md#tp-theme-changed-event: TP-T10 — theme_changed event で System → media query 監視再開', async () => {
		const cap = makeListenCapture();
		const { fn, mql } = makeMatchMediaFn(false);
		const el = makeElement();
		const subscriber = createThemeSubscriber({
			listenFn: cap.listenFn,
			matchMediaFn: fn,
			documentElement: el
		});

		subscriber.setTheme('Dark');
		await subscriber.start();
		cap.emit(makePayload({ new_theme: 'System' }));

		expect(el.classList.contains('dark')).toBe(false);

		mql.matches = true;
		mql.dispatch();

		expect(el.classList.contains('dark')).toBe(true);
	});

	it('spec.md#tp-theme-changed-event: TP-T11 — 非 Tauri 環境で event 購読 silent fallback 後も setTheme が機能する', async () => {
		const failingListen: ThemeListener = async () => {
			throw new Error('boom');
		};
		const { fn } = makeMatchMediaFn(false);
		const el = makeElement();
		const subscriber = createThemeSubscriber({
			listenFn: failingListen,
			matchMediaFn: fn,
			documentElement: el
		});

		await expect(subscriber.start()).resolves.toBeUndefined();

		subscriber.setTheme('Dark');
		expect(el.classList.contains('dark')).toBe(true);
	});

	it('lifecycle — stop で event unsubscribe と media listener remove が呼ばれる', async () => {
		const cap = makeListenCapture();
		const { fn, mql } = makeMatchMediaFn(false);
		const removeSpy = vi.spyOn(mql, 'removeEventListener');
		const el = makeElement();
		const subscriber = createThemeSubscriber({
			listenFn: cap.listenFn,
			matchMediaFn: fn,
			documentElement: el
		});

		await subscriber.start();
		subscriber.setTheme('System');
		subscriber.stop();

		expect(cap.unsubscribe).toHaveBeenCalledTimes(1);
		expect(removeSpy).toHaveBeenCalled();
	});

	it('lifecycle — stop 後は media query 変化が反映されない', async () => {
		const { fn, mql } = makeMatchMediaFn(false);
		const el = makeElement();
		const subscriber = createThemeSubscriber({ matchMediaFn: fn, documentElement: el });

		subscriber.setTheme('System');
		subscriber.stop();

		mql.matches = true;
		mql.dispatch();

		expect(el.classList.contains('dark')).toBe(false);
	});

	it('divergence — onThemeChanged callback が theme_changed event で呼ばれる', async () => {
		const cap = makeListenCapture();
		const { fn } = makeMatchMediaFn(false);
		const onThemeChanged = vi.fn();
		const subscriber = createThemeSubscriber({
			listenFn: cap.listenFn,
			matchMediaFn: fn,
			documentElement: makeElement(),
			onThemeChanged
		});

		await subscriber.start();
		cap.emit(makePayload({ new_theme: 'Dark' }));

		expect(onThemeChanged).toHaveBeenCalledExactlyOnceWith('Dark');
	});
});
