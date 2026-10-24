import { invoke } from '@tauri-apps/api/core';

export type ReleaseResponse = {
	version: string;
	url: string;
	notes: string;
};

export type UpdateChannelResponse = {
	current_version: string;
	latest_release: ReleaseResponse | null;
};

/**
 * Invoke the `check_for_updates` Tauri command.
 *
 * I-U3: アプリ起動時に 1 回だけ呼ぶ (page-main の mount-time effect)。polling / retry はしない。
 * 戻り値は常に `UpdateChannelResponse` (C-CFU1: no Result) — 失敗は Rust 側で
 * `latest_release: null` に降格される (S14 silent failure)。新版検出時は Rust 側が
 * `new_version_detected` event を emit するので、呼び出し前に widget-update-toast の
 * listen を確立しておくこと。
 */
export async function checkForUpdates(): Promise<UpdateChannelResponse> {
	return invoke<UpdateChannelResponse>('check_for_updates');
}
