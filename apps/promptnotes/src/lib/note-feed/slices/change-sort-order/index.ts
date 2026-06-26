import { invoke } from '@tauri-apps/api/core';

export type SortField = 'created_at' | 'updated_at';
export type SortDirection = 'asc' | 'desc';

export type SortOrder = {
	field: SortField;
	direction: SortDirection;
};

export type ChangeSortOrderInput = {
	new_sort: SortOrder;
};

export type NoteFeedDto = {
	sort: SortOrder;
};

export type ChangeSortOrderError =
	| { kind: 'persist_error'; path: string; reason: string }
	| { kind: 'invalid_path'; path: string; reason: 'not_absolute' | 'contains_config_path' };

/**
 * Invoke the `change_sort_order` Tauri command.
 *
 * Atomically updates the in-memory NoteFeed sort and persists the new
 * `sort_preference` to `settings.json` via `SettingsRepository::save`
 * (NoteFeed → Settings の唯一の逆流, `aggregates.md#notes-sort-side-effect`).
 *
 * Side effects on success (C-CSO1: same-value input is a no-op):
 * - persists `settings.json`
 * - emits `settings:sort_preference_changed` to the Tauri event bus
 */
export async function changeSortOrder(input: ChangeSortOrderInput): Promise<NoteFeedDto> {
	return invoke<NoteFeedDto>('change_sort_order', { input });
}
