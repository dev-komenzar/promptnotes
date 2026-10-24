import { listen as tauriListen } from '@tauri-apps/api/event';
import { openUrl } from '@tauri-apps/plugin-opener';

/**
 * widget-update-toast の購読 + dismiss / view-release ロジックを保持する store。
 *
 * - I-UT1: page-main mount 時に listen を 1 回張り、unmount で解除
 * - I-UT2: NewVersionDetected 未発行時は payload=null のまま（silent, S14）
 * - I-UT3: 同セッション中の再 emit は最新で payload を上書き
 * - I-UT4: dismiss は payload=null（次回起動で再 emit があれば再表示）
 * - I-UT5: view-release は openUrlFn を呼び、payload は維持
 * - onSubscribed: listen 確立 **後** に 1 回だけ呼ぶ hook。page-main はここで起動時の
 *   check-for-updates (I-U3) を invoke する。listen 前に emit されると取りこぼすため、
 *   check は必ず購読確立後に走らせる (ori-m2ra.1)
 */

export type NewVersionDetectedPayload = {
	current_version: string;
	latest_version: string;
	release_url: string;
	release_notes: string;
};

export type UpdateEventListener = (
	handler: (payload: NewVersionDetectedPayload) => void
) => Promise<() => void>;

export type UpdateUrlOpener = (url: string) => void | Promise<void>;

export type UpdateSubscribedHook = () => void | Promise<void>;

export type UpdateToastStoreDeps = {
	listenFn?: UpdateEventListener;
	openUrlFn?: UpdateUrlOpener;
	onSubscribed?: UpdateSubscribedHook;
};

export type UpdateToastStore = ReturnType<typeof createUpdateToastStore>;

const NEW_VERSION_DETECTED_EVENT = 'new_version_detected';

const defaultListenFn: UpdateEventListener = async (handler) =>
	tauriListen<NewVersionDetectedPayload>(NEW_VERSION_DETECTED_EVENT, (event) => {
		handler(event.payload);
	});

// I-UT7: Tauri webview の window.open は OS ブラウザを開かないため opener plugin を使う。
const defaultOpenUrlFn: UpdateUrlOpener = async (url) => {
	try {
		await openUrl(url);
	} catch {
		// silent — non-Tauri host / opener 失敗時も toast は維持する
	}
};

export function createUpdateToastStore(deps: UpdateToastStoreDeps = {}) {
	const listenFn = deps.listenFn ?? defaultListenFn;
	const openUrlFn = deps.openUrlFn ?? defaultOpenUrlFn;
	const onSubscribed = deps.onSubscribed;

	let payload = $state<NewVersionDetectedPayload | null>(null);
	let unsubscribe: (() => void) | null = null;

	async function start(): Promise<void> {
		if (unsubscribe) return;
		try {
			unsubscribe = await listenFn((p) => {
				payload = p;
			});
		} catch {
			// I-UT2 / S14: listen 失敗は silent。toast は出さない。
			return;
		}
		try {
			await onSubscribed?.();
		} catch {
			// S14: 起動時 check の失敗は silent。
		}
	}

	function stop(): void {
		unsubscribe?.();
		unsubscribe = null;
	}

	function dismiss(): void {
		payload = null;
	}

	async function viewRelease(): Promise<void> {
		if (!payload) return;
		await openUrlFn(payload.release_url);
	}

	return {
		get payload() {
			return payload;
		},
		start,
		stop,
		dismiss,
		viewRelease
	};
}
