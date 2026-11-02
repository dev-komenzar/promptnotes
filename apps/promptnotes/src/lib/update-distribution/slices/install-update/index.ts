import { invoke } from '@tauri-apps/api/core';

/** Rust `InstallUpdateError` の variant 名 (spec.md#io-errors)。 */
export type InstallUpdateErrorKind =
	| 'UnsupportedInstallation'
	| 'NoUpdateAvailable'
	| 'InstallInProgress'
	| 'CheckFailed'
	| 'DownloadFailed'
	| 'InstallFailed';

export type InstallUpdateErrorResponse = {
	kind: InstallUpdateErrorKind;
	message: string;
};

/**
 * 最新リリースを取得し直して download + 署名検証 + install する (I-U5〜I-U7)。
 * resolve 値はインストールしたバージョン。失敗時は `InstallUpdateErrorResponse` で reject する。
 * 再起動はしない — 呼出側が全 pending Note を Flush してから `restartApp` を呼ぶ (I-U8)。
 */
export async function installUpdate(): Promise<string> {
	return invoke<string>('install_update');
}

/**
 * reject 値が「別のインストールが実行中 / 再起動待ち」(I-U10) か。toast と設定モーダルは独立に
 * install を要求できるが、Rust 側が同時実行を 1 本に制限して 2 本目をこの kind で拒否する。
 */
export function isInstallInProgressError(error: unknown): boolean {
	return (
		typeof error === 'object' &&
		error !== null &&
		(error as { kind?: unknown }).kind === 'InstallInProgress'
	);
}

/** 通知 UI が「Update & restart」を出してよいか (I-U6)。ネットワークには触れない。 */
export async function isUpdateInstallable(): Promise<boolean> {
	return invoke<boolean>('is_update_installable');
}

/** アプリを再起動する。install 成功 + Flush 完了後にだけ呼ぶ (I-U8)。 */
export async function restartApp(): Promise<void> {
	await invoke('restart_app');
}
