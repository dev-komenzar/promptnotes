import { invoke } from '@tauri-apps/api/core';

/**
 * Invoke the `get_app_version` Tauri command.
 *
 * ビルド時に埋め込まれた現在のアプリバージョンを返す（表示用）。
 * 戻り値は常に `string` (C-GAV1: no Result) — pre-release 等で semver として
 * parse できない場合は raw 文字列がそのまま返る。
 */
export async function getAppVersion(): Promise<string> {
	return invoke<string>('get_app_version');
}
