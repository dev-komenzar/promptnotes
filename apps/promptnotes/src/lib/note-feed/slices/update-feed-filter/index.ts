import { invoke } from '@tauri-apps/api/core';

export type DateRangeFilter =
	| { kind: 'last_7_days' }
	| { kind: 'last_30_days' }
	| { kind: 'last_90_days' }
	| { kind: 'all' }
	| { kind: 'custom'; from: string; to: string };

export type UpdateFeedFilterInput =
	| { kind: 'set_query'; raw: string }
	| { kind: 'set_date_range'; range: DateRangeFilter }
	/** `raw = null` で tag filter を解除する (C-UF4 と独立、SetTag(None))。 */
	| { kind: 'set_tag'; raw: string | null }
	| { kind: 'clear_all' };

export type NoteFeedFilterDto = {
	query: string | null;
	date_range: DateRangeFilter;
	tag: string | null;
};

export type UpdateFeedFilterError = {
	kind: 'invalid_tag';
	raw: string;
	reason: 'invalid_char' | 'empty';
};

/**
 * Invoke the `update_feed_filter` Tauri command.
 *
 * NoteFeed の filter (query / date_range / tag) を 1 命令で更新する。副作用ゼロ
 * (C-UF6) の use case を Tauri から呼ぶ薄い wrapper で、唯一の副作用は
 * `InMemoryNoteFeedState` の差し替え (揮発、event 非発行)。同値再適用は冪等
 * (C-UF3 / C-UF7)、`clear_all` は I-F6 初期状態へリセット (C-UF4)。
 *
 * `set_tag.raw` は Tag::new で I-N6 (禁止文字 / 空文字) を検証するため、
 * 失敗時は [`UpdateFeedFilterError`] を返す。
 */
export async function updateFeedFilter(
	input: UpdateFeedFilterInput
): Promise<NoteFeedFilterDto> {
	return invoke<NoteFeedFilterDto>('update_feed_filter', { input });
}
