import { invoke } from '@tauri-apps/api/core';

export type Theme = 'System' | 'Light' | 'Dark';

export type SortField = 'created_at' | 'updated_at';

export type SortDirection = 'asc' | 'desc';

export type SortOrder = {
	field: SortField;
	direction: SortDirection;
};

/**
 * Settings aggregate root の DTO 表現。`storage_dir` は `#[serde(transparent)]`
 * で生 path 文字列に展開される。
 */
export type Settings = {
	storage_dir: string;
	theme: Theme;
	sort_preference: SortOrder;
};

/**
 * Invoke the `load_settings` Tauri command.
 *
 * 起動時に `app_config_dir/settings.json` を読んで Settings を復元する。
 * 戻り値は常に `Settings` (C-LS1: no Result) — 失敗時は I-S3 defaults へ
 * 降格、ensure_dir 失敗も silent。
 */
export async function loadSettings(): Promise<Settings> {
	return invoke<Settings>('load_settings');
}
