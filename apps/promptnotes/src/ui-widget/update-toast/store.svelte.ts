import { listen as tauriListen } from '@tauri-apps/api/event';
import { openUrl } from '@tauri-apps/plugin-opener';
import {
	installUpdate,
	isInstallInProgressError,
	isUpdateInstallable
} from '$lib/update-distribution/slices/install-update';

/**
 * widget-update-toast の購読 + dismiss / view-release ロジックを保持する store。
 *
 * - I-UT1: page-main mount 時に listen を 1 回張り、unmount で解除
 * - I-UT2: NewVersionDetected 未発行時は payload=null のまま（silent, S14）
 * - I-UT3: 同セッション中の再 emit (24 時間ごとの定期確認で新しいバージョンを検出) は最新で payload を上書き
 * - I-UT4: dismiss は payload=null。同じバージョンは同一プロセス内で再 emit されない (I-U4) ので
 *   再表示されない。次回起動時、または定期確認でより新しいバージョンが emit されたら再表示
 * - I-UT5: view-release は openUrlFn を呼び、payload は維持
 * - onSubscribed: listen 確立 **後** に 1 回だけ呼ぶ hook。page-main はここで起動時の
 *   check-for-updates (I-U3) を invoke する。listen 前に emit されると取りこぼすため、
 *   check は必ず購読確立後に走らせる (ori-m2ra.1)
 * - I-UT9 (I-U6): start 時に installableFn で「Update & restart」を出せるか判定する。失敗時は false
 *   (通知のみ)
 * - I-UT10 (I-U5 / S24): install は installStatus=installing の間は多重実行しない。成功したら
 *   onInstalled (page-main: 全 Note Flush → 再起動、I-U8) を await する
 * - I-UT11 (I-U7 / S25): install 失敗は installStatus=failed にして再実行可能に戻す。payload は維持
 * - I-UT12 (I-U10 / S29): 別 UI (設定モーダル) のインストールが実行中 / 再起動待ちで拒否されたら
 *   installStatus=already-installing。失敗扱いにしない
 * - I-UT13 (I-U8 / S29): install 成功後に onInstalled (再起動) が reject したら installStatus=restart-required。
 *   dismiss を有効に戻し、再インストールは受け付けない
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

export type UpdateInstallableFn = () => Promise<boolean>;

export type UpdateInstallFn = () => Promise<string>;

/** install 成功後に呼ぶ hook。page-main が全 pending Note を Flush してから再起動する (I-U8)。 */
export type UpdateInstalledHook = (installedVersion: string) => void | Promise<void>;

export type InstallStatus =
	| 'idle'
	| 'installing'
	| 'failed'
	| 'already-installing'
	| 'restart-required';

export type UpdateToastStoreDeps = {
	listenFn?: UpdateEventListener;
	openUrlFn?: UpdateUrlOpener;
	onSubscribed?: UpdateSubscribedHook;
	installableFn?: UpdateInstallableFn;
	installFn?: UpdateInstallFn;
	onInstalled?: UpdateInstalledHook;
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
	const installableFn = deps.installableFn ?? isUpdateInstallable;
	const installFn = deps.installFn ?? installUpdate;
	const onInstalled = deps.onInstalled;

	let payload = $state<NewVersionDetectedPayload | null>(null);
	let installable = $state(false);
	let installStatus = $state<InstallStatus>('idle');
	let unsubscribe: (() => void) | null = null;

	async function start(): Promise<void> {
		if (unsubscribe) return;
		try {
			unsubscribe = await listenFn((p) => {
				payload = p;
				// より新しい版の通知で失敗表示はリセットする。install 中は状態を変えない。
				if (installStatus === 'failed' || installStatus === 'already-installing') {
					installStatus = 'idle';
				}
			});
		} catch {
			// I-UT2 / S14: listen 失敗は silent。toast は出さない。
			return;
		}
		try {
			installable = await installableFn();
		} catch {
			// I-UT9: 判定できなければ通知のみ (View release) にとどめる。
			installable = false;
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
		if (installStatus === 'installing') return;
		payload = null;
		installStatus = 'idle';
	}

	async function install(): Promise<void> {
		if (
			!payload ||
			!installable ||
			installStatus === 'installing' ||
			installStatus === 'restart-required'
		)
			return;
		installStatus = 'installing';
		let installedVersion: string;
		try {
			installedVersion = await installFn();
		} catch (err) {
			if (isInstallInProgressError(err)) {
				installStatus = 'already-installing';
				return;
			}
			console.error('[update] install failed', err);
			installStatus = 'failed';
			return;
		}
		// 成功時は onInstalled (Flush → 再起動) に進む。再起動までは installing のまま。
		try {
			await onInstalled?.(installedVersion);
		} catch (err) {
			console.error('[update] restart after install failed', err);
			installStatus = 'restart-required';
		}
	}

	async function viewRelease(): Promise<void> {
		if (!payload) return;
		await openUrlFn(payload.release_url);
	}

	return {
		get payload() {
			return payload;
		},
		get installable() {
			return installable;
		},
		get installStatus() {
			return installStatus;
		},
		start,
		stop,
		dismiss,
		install,
		viewRelease
	};
}
