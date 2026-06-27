<script lang="ts">
	import type { DateRangeFilter } from '$lib/note-feed/slices/update-feed-filter';
	import { feedStore } from '../stores/feed.svelte';

	type Props = {
		onOpenSettings?: () => void;
	};

	let { onOpenSettings }: Props = $props();

	const DATE_PRESETS: Array<{
		key: DateRangeFilter['kind'];
		label: string;
		range: DateRangeFilter;
	}> = [
		{ key: 'all', label: 'すべて', range: { kind: 'all' } },
		{ key: 'last_7_days', label: '7 日', range: { kind: 'last_7_days' } },
		{ key: 'last_30_days', label: '30 日', range: { kind: 'last_30_days' } },
		{ key: 'last_90_days', label: '90 日', range: { kind: 'last_90_days' } }
	];

	let queryDraft = $state('');

	$effect(() => {
		const next = feedStore.filter.query ?? '';
		if (next !== queryDraft) {
			queryDraft = next;
		}
	});

	function handleQueryInput(event: Event) {
		const value = (event.currentTarget as HTMLInputElement).value;
		queryDraft = value;
		void feedStore.setQuery(value);
	}

	function selectDateRange(range: DateRangeFilter) {
		void feedStore.setDateRange(range);
	}

	function clearTag() {
		void feedStore.setTag(null);
	}

	function selectSortField(field: 'created_at' | 'updated_at') {
		if (feedStore.sort.field === field) return;
		void feedStore.setSortField(field);
	}

	function toggleSortDirection() {
		const next = feedStore.sort.direction === 'asc' ? 'desc' : 'asc';
		void feedStore.setSortDirection(next);
	}

	function clearAll() {
		queryDraft = '';
		void feedStore.clearAll();
	}

	function openSettings() {
		onOpenSettings?.();
	}
</script>

<header
	data-testid="region-toolbar"
	aria-label="ツールバー"
	class="flex shrink-0 flex-wrap items-center gap-2 border-b border-neutral-200 bg-neutral-50/80 px-3 py-2 text-sm text-neutral-700 backdrop-blur dark:border-neutral-800 dark:bg-neutral-900/80 dark:text-neutral-200"
>
	<label class="flex min-w-0 flex-1 items-center gap-1.5" for="screen-1-toolbar-search-query">
		<span aria-hidden="true" class="text-neutral-400">🔍</span>
		<input
			id="screen-1-toolbar-search-query"
			data-testid="screen-1-toolbar-search-query"
			type="search"
			class="w-full min-w-0 rounded-md border border-neutral-200 bg-white px-2 py-1 text-sm placeholder:text-neutral-400 focus:border-blue-500 focus:outline-none dark:border-neutral-700 dark:bg-neutral-800"
			placeholder="検索 (Cmd+F)"
			aria-label="検索"
			value={queryDraft}
			oninput={handleQueryInput}
		/>
	</label>

	<div
		data-testid="screen-1-toolbar-date-range"
		role="group"
		aria-label="期間"
		class="inline-flex shrink-0 overflow-hidden rounded-md border border-neutral-200 dark:border-neutral-700"
	>
		{#each DATE_PRESETS as preset (preset.key)}
			{@const active = feedStore.filter.date_range.kind === preset.key}
			<button
				type="button"
				data-testid={`screen-1-toolbar-date-range-${preset.key}`}
				class={[
					'px-2 py-1 text-xs transition-colors',
					active
						? 'bg-blue-600 text-white'
						: 'bg-white text-neutral-600 hover:bg-neutral-100 dark:bg-neutral-800 dark:text-neutral-300 dark:hover:bg-neutral-700'
				]}
				aria-pressed={active}
				onclick={() => selectDateRange(preset.range)}
			>
				{preset.label}
			</button>
		{/each}
	</div>

	{#if feedStore.filter.tag}
		<button
			type="button"
			data-testid="screen-1-toolbar-tag-chip"
			class="inline-flex shrink-0 items-center gap-1 rounded-full bg-blue-100 px-2 py-0.5 text-xs text-blue-800 hover:bg-blue-200 dark:bg-blue-900/40 dark:text-blue-200 dark:hover:bg-blue-900/60"
			aria-label={`タグ ${feedStore.filter.tag} を解除`}
			onclick={clearTag}
		>
			<span>#{feedStore.filter.tag}</span>
			<span aria-hidden="true">×</span>
		</button>
	{/if}

	<div
		role="group"
		aria-label="ソート対象"
		data-testid="screen-1-toolbar-sort-field"
		class="inline-flex shrink-0 overflow-hidden rounded-md border border-neutral-200 dark:border-neutral-700"
	>
		<button
			type="button"
			data-testid="screen-1-toolbar-sort-field-created_at"
			class={[
				'px-2 py-1 text-xs transition-colors',
				feedStore.sort.field === 'created_at'
					? 'bg-neutral-200 text-neutral-900 dark:bg-neutral-700 dark:text-neutral-50'
					: 'bg-white text-neutral-600 hover:bg-neutral-100 dark:bg-neutral-800 dark:text-neutral-300 dark:hover:bg-neutral-700'
			]}
			aria-pressed={feedStore.sort.field === 'created_at'}
			onclick={() => selectSortField('created_at')}
		>
			作成日
		</button>
		<button
			type="button"
			data-testid="screen-1-toolbar-sort-field-updated_at"
			class={[
				'px-2 py-1 text-xs transition-colors',
				feedStore.sort.field === 'updated_at'
					? 'bg-neutral-200 text-neutral-900 dark:bg-neutral-700 dark:text-neutral-50'
					: 'bg-white text-neutral-600 hover:bg-neutral-100 dark:bg-neutral-800 dark:text-neutral-300 dark:hover:bg-neutral-700'
			]}
			aria-pressed={feedStore.sort.field === 'updated_at'}
			onclick={() => selectSortField('updated_at')}
		>
			更新日
		</button>
	</div>

	<button
		type="button"
		data-testid="screen-1-toolbar-sort-direction"
		class="inline-flex shrink-0 items-center gap-1 rounded-md border border-neutral-200 bg-white px-2 py-1 text-xs hover:bg-neutral-100 dark:border-neutral-700 dark:bg-neutral-800 dark:hover:bg-neutral-700"
		aria-label={feedStore.sort.direction === 'asc' ? '昇順' : '降順'}
		onclick={toggleSortDirection}
	>
		<span aria-hidden="true">{feedStore.sort.direction === 'asc' ? '↑' : '↓'}</span>
	</button>

	<button
		type="button"
		data-testid="screen-1-toolbar-clear-all"
		class="shrink-0 rounded-md border border-neutral-200 bg-white px-2 py-1 text-xs text-neutral-600 hover:bg-neutral-100 dark:border-neutral-700 dark:bg-neutral-800 dark:text-neutral-300 dark:hover:bg-neutral-700"
		aria-label="フィルターをすべて解除"
		onclick={clearAll}
	>
		ClearAll
	</button>

	<button
		type="button"
		data-testid="screen-1-toolbar-settings-button"
		class="shrink-0 rounded-md border border-transparent px-2 py-1 text-base hover:bg-neutral-200/60 dark:hover:bg-neutral-700/60"
		aria-label="設定を開く"
		onclick={openSettings}
	>
		<span aria-hidden="true">⚙️</span>
	</button>
</header>
