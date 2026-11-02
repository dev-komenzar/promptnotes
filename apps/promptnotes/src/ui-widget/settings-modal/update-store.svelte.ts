import { openUrl } from '@tauri-apps/plugin-opener';
import {
	checkForUpdatesManually as defaultCheckForUpdates,
	type ReleaseResponse
} from '$lib/update-distribution/slices/check-for-updates-manually';
import {
	installUpdate as defaultInstallUpdate,
	isUpdateInstallable as defaultIsUpdateInstallable
} from '$lib/update-distribution/slices/install-update';

/**
 * widget-settings-modal の「Updates」セクション (手動確認 + インストール) の state を保持する store。
 * form の draft (storage_dir / theme) とは独立 (I-SM17: dirty / save / cancel rollback の対象外)。
 *
 * - I-SM15: 確認は `check-for-updates-manually` slice 経由で、`check()` 呼出時のみ (mount 時は呼ばない)
 * - I-SM16: 確認中 / インストール中の `check()` は no-op
 * - I-SM17: 結果は idle / checking / up-to-date / available / failed。Toast は出さない (I-U9)
 * - I-SM18: `loadInstallable()` で `isUpdateInstallable` を 1 回判定。失敗は false (View release のみ)
 * - I-SM19: `install()` 成功後に `onInstalled` (page-main: 全 Note Flush → 再起動、I-U8) を await
 * - I-SM20: インストール中は `busy` (modal は閉じない・他の操作を受け付けない)。成功後も再起動まで installing
 * - I-SM21: インストール失敗は `failed` にして再試行可能に戻す。確認結果は維持、再確認で失敗表示を消す
 */

export type CheckStatus =
	| { kind: 'idle' }
	| { kind: 'checking' }
	| { kind: 'up-to-date' }
	| { kind: 'available'; release: ReleaseResponse }
	| { kind: 'failed' };

export type InstallStatus = 'idle' | 'installing' | 'failed';

export type SettingsUpdateStoreDeps = {
	checkForUpdatesFn?: typeof defaultCheckForUpdates;
	installableFn?: typeof defaultIsUpdateInstallable;
	installFn?: typeof defaultInstallUpdate;
	openUrlFn?: (url: string) => void | Promise<void>;
	/** install 成功後に呼ぶ hook。page-main が全 pending Note を Flush してから再起動する (I-U8)。 */
	onInstalled?: (installedVersion: string) => void | Promise<void>;
};

export type SettingsUpdateStore = ReturnType<typeof createSettingsUpdateStore>;

// Tauri webview の window.open は OS ブラウザを開かないため opener plugin を使う (widget-update-toast と同じ)。
const defaultOpenUrlFn = async (url: string): Promise<void> => {
	try {
		await openUrl(url);
	} catch {
		// silent — non-Tauri host / opener 失敗時も modal は維持する
	}
};

export function createSettingsUpdateStore(deps: SettingsUpdateStoreDeps = {}) {
	const checkForUpdatesFn = deps.checkForUpdatesFn ?? defaultCheckForUpdates;
	const installableFn = deps.installableFn ?? defaultIsUpdateInstallable;
	const installFn = deps.installFn ?? defaultInstallUpdate;
	const openUrlFn = deps.openUrlFn ?? defaultOpenUrlFn;
	const onInstalled = deps.onInstalled;

	let checkStatus = $state<CheckStatus>({ kind: 'idle' });
	let installable = $state(false);
	let installStatus = $state<InstallStatus>('idle');

	const busy = $derived(installStatus === 'installing');

	async function loadInstallable(): Promise<void> {
		try {
			installable = await installableFn();
		} catch {
			installable = false;
		}
	}

	async function check(): Promise<void> {
		if (checkStatus.kind === 'checking' || busy) return;
		checkStatus = { kind: 'checking' };
		installStatus = 'idle';
		try {
			const result = await checkForUpdatesFn();
			checkStatus = result.latest_release
				? { kind: 'available', release: result.latest_release }
				: { kind: 'up-to-date' };
		} catch (err) {
			console.error('[update] manual check failed', err);
			checkStatus = { kind: 'failed' };
		}
	}

	async function install(): Promise<void> {
		if (checkStatus.kind !== 'available' || !installable || busy) return;
		installStatus = 'installing';
		let installedVersion: string;
		try {
			installedVersion = await installFn();
		} catch (err) {
			console.error('[update] install failed', err);
			installStatus = 'failed';
			return;
		}
		// 成功時は onInstalled (Flush → 再起動) に進む。再起動までは installing のまま。
		await onInstalled?.(installedVersion);
	}

	async function viewRelease(): Promise<void> {
		if (checkStatus.kind !== 'available') return;
		await openUrlFn(checkStatus.release.url);
	}

	return {
		get checkStatus() {
			return checkStatus;
		},
		get installable() {
			return installable;
		},
		get installStatus() {
			return installStatus;
		},
		get busy() {
			return busy;
		},
		loadInstallable,
		check,
		install,
		viewRelease
	};
}
