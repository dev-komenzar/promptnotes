import { invoke } from '@tauri-apps/api/core';

export type Theme = 'System' | 'Light' | 'Dark';

export type SortField = 'created_at' | 'updated_at';
export type SortDirection = 'asc' | 'desc';

export type SortOrder = {
	field: SortField;
	direction: SortDirection;
};

export type SettingsDto = {
	storage_dir: string;
	theme: Theme;
	sort_preference: SortOrder;
};

export type UpdateSettingsInput = {
	storage_dir?: string;
	theme?: Theme;
};

export type UpdateSettingsError =
	| {
			kind: 'invalid_path';
			path: string;
			reason: 'not_absolute' | 'contains_config_path';
	  }
	| { kind: 'persist_error'; path: string; reason: string };

/**
 * Invoke the `update_settings` Tauri command. Resolves with the updated
 * `SettingsDto` (no-op input still returns current `Settings`, C-US1/C-US2)
 * and rejects with `UpdateSettingsError` mirroring spec.md#io-errors.
 *
 * Side effects on success (diff-conditional, C-US5):
 * - persists `settings.json` via `SettingsRepository::save`
 * - emits `settings:storage_dir_changed` / `settings:theme_changed`
 *   events to the Tauri event bus
 */
export async function updateSettings(input: UpdateSettingsInput): Promise<SettingsDto> {
	return invoke<SettingsDto>('update_settings', { input });
}
