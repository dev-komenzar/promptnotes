import { invoke } from '@tauri-apps/api/core';

export type NoteSummaryDto = {
	id: string;
	body: string;
	tags: string[];
	created_at: string;
	updated_at: string;
};

export type NoteFeedDto = {
	notes: NoteSummaryDto[];
};

/**
 * Invoke the `list_notes` Tauri command.
 *
 * 起動時 / 手動 Refresh で呼び出す read pipeline。`storage_dir/*.md` を
 * 全件 hydration し、現在の filter + sort (settings.json から復元) を
 * 適用した `visible_notes` を返す (`workflows/list-feed.md`)。
 *
 * Pipeline:
 *   1. Settings から storage_dir を解決 (load-settings 経路を再利用)
 *   2. FsNoteRepository::list_all() で .md 全件を Note へ parse (個別 skip on error)
 *   3. NoteFeed.source を hydrate (Vec<Note> 採用、aggregates.md#note-feed-aggregate-elements)
 *   4. filter (現在の InMemoryNoteFeedState) + sort (Settings.sort_preference) を適用
 *   5. visible_notes を NoteSummaryDto[] に投影して返す
 *
 * 副作用は `InMemoryNoteFeedState::replace` のみ (read 側、揮発、event 発行なし)。
 * Rust 側の I/O 失敗 / parse 失敗は port 内で log + skip するため、上に伝わる
 * `Result` は `read_dir` の致命的失敗のみ。UI 側は silent fallback で受ける。
 */
export async function listNotes(): Promise<NoteFeedDto> {
	return invoke<NoteFeedDto>('list_notes');
}
