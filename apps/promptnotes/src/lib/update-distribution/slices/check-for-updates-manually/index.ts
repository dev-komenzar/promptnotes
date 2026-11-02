import { invoke } from '@tauri-apps/api/core';
import type { UpdateChannelResponse } from '$lib/update-distribution/slices/check-for-updates';

export type {
	ReleaseResponse,
	UpdateChannelResponse
} from '$lib/update-distribution/slices/check-for-updates';

/** Rust `UpdateError` の variant 名 (spec.md#io-errors)。 */
export type UpdateCheckErrorKind = 'NetworkError' | 'ParseError' | 'RateLimited';

export type UpdateCheckErrorResponse = {
	kind: UpdateCheckErrorKind;
	message: string;
};

/**
 * 設定モーダルの「Check for updates」から呼ぶ手動確認 (I-U9)。
 *
 * resolve 値の `latest_release` が非 null なら新しいリリースあり、null なら最新 (I-U2)。
 * 確認失敗は `UpdateCheckErrorResponse` で reject する (silent にしない)。
 * `new_version_detected` event は発行されず、自動確認の前回確認時刻 / 通知済みバージョンも変わらない。
 */
export async function checkForUpdatesManually(): Promise<UpdateChannelResponse> {
	return invoke<UpdateChannelResponse>('check_for_updates_manually');
}
