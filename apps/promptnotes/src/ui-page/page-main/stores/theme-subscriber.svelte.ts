import { listen as tauriListen } from '@tauri-apps/api/event';
import type { Theme } from '$lib/user-preferences/slices/load-settings';

export type ThemeChangedPayload = {
	new_theme: Theme;
};

export type ThemeListener = (
	handler: (payload: ThemeChangedPayload) => void
) => Promise<() => void>;

export type MatchMediaFn = (query: string) => MediaQueryList;

export type ThemeSubscriberDeps = {
	listenFn?: ThemeListener;
	matchMediaFn?: MatchMediaFn;
	documentElement?: HTMLElement;
	onThemeChanged?: (theme: Theme) => void;
};

const THEME_CHANGED_EVENT = 'settings:theme_changed';
const DARK_QUERY = '(prefers-color-scheme: dark)';

const defaultListenFn: ThemeListener = async (handler) =>
	tauriListen<ThemeChangedPayload>(THEME_CHANGED_EVENT, (event) => {
		handler(event.payload);
	});

function effectiveDark(theme: Theme, matches: boolean): boolean {
	if (theme === 'Dark') return true;
	if (theme === 'Light') return false;
	return matches;
}

export function createThemeSubscriber(deps: ThemeSubscriberDeps = {}) {
	const listenFn = deps.listenFn ?? defaultListenFn;
	const matchMediaFn = deps.matchMediaFn ?? ((q: string) => window.matchMedia(q));
	const documentEl = deps.documentElement ?? document.documentElement;

	let currentTheme: Theme = 'System';
	const mql = matchMediaFn(DARK_QUERY);
	let mediaHandler: (() => void) | null = null;
	let eventUnlisten: (() => void) | null = null;

	function apply(): void {
		const dark = effectiveDark(currentTheme, mql.matches);
		documentEl.classList.toggle('dark', dark);
	}

	function setTheme(theme: Theme): void {
		currentTheme = theme;
		if (theme === 'System' && !mediaHandler) {
			mediaHandler = () => apply();
			mql.addEventListener('change', mediaHandler);
		} else if (theme !== 'System' && mediaHandler) {
			mql.removeEventListener('change', mediaHandler);
			mediaHandler = null;
		}
		apply();
	}

	async function start(): Promise<void> {
		if (eventUnlisten) return;
		try {
			eventUnlisten = await listenFn((payload) => {
				setTheme(payload.new_theme);
				deps.onThemeChanged?.(payload.new_theme);
			});
		} catch {
			// non-Tauri host (test / browser) は theme_changed event 不要
		}
	}

	function stop(): void {
		eventUnlisten?.();
		eventUnlisten = null;
		if (mediaHandler) {
			mql.removeEventListener('change', mediaHandler);
			mediaHandler = null;
		}
	}

	return { setTheme, start, stop };
}
