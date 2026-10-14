import { invoke } from '@tauri-apps/api/core';

/**
 * Invoke the `get_default_storage_dir` Tauri command.
 *
 * OS 慣習に基づく default `storage_dir`（I-S3、`<app_data_dir>/notes`）を絶対パス文字列で返す。
 * 読み取り専用 query で、`settings.json` の読み書きや event 発行はしない（C-GDSD2）。
 * Rust 側は `Result` を返さない（C-GDSD1）が、IPC 失敗時は reject しうる。
 */
export async function getDefaultStorageDir(): Promise<string> {
	return invoke<string>('get_default_storage_dir');
}
