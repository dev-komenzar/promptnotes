import { listen as tauriListen } from '@tauri-apps/api/event';

/**
 * widget-update-toast の購読 + dismiss / view-release ロジックを保持する store。
 *
 * - I-UT1: page-main mount 時に listen を 1 回張り、unmount で解除
 * - I-UT2: NewVersionDetected 未発行時は payload=null のまま（silent, S14）
 * - I-UT3: 同セッション中の再 emit は最新で payload を上書き
 * - I-UT4: dismiss は payload=null（次回起動で再 emit があれば再表示）
 * - I-UT5: view-release は openUrlFn を呼び、payload は維持
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

export type UpdateToastStoreDeps = {
	listenFn?: UpdateEventListener;
	openUrlFn?: UpdateUrlOpener;
};

export type UpdateToastStore = ReturnType<typeof createUpdateToastStore>;

const NEW_VERSION_DETECTED_EVENT = 'new_version_detected';

const defaultListenFn: UpdateEventListener = async (handler) =>
	tauriListen<NewVersionDetectedPayload>(NEW_VERSION_DETECTED_EVENT, (event) => {
		handler(event.payload);
	});

const defaultOpenUrlFn: UpdateUrlOpener = (url) => {
	if (typeof window !== 'undefined') {
		window.open(url, '_blank', 'noopener');
	}
};

export function createUpdateToastStore(deps: UpdateToastStoreDeps = {}) {
	const listenFn = deps.listenFn ?? defaultListenFn;
	const openUrlFn = deps.openUrlFn ?? defaultOpenUrlFn;

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
