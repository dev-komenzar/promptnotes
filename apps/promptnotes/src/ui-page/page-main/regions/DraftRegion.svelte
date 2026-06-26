<script lang="ts">
	// page-main-shell では中身は placeholder のみ。
	// CodeMirror + create-note 連携は page-main-draft sub-task (ori-054.3) で実装する。
</script>

<section
	data-testid="region-draft"
	aria-label="新規 Note"
	class="sticky top-0 z-10 shrink-0 border-b border-neutral-200 bg-white px-3 py-2 dark:border-neutral-800 dark:bg-neutral-950"
>
	<div
		class="flex min-h-[3rem] items-center rounded border border-dashed border-neutral-300 px-3 py-2 text-sm text-neutral-400 dark:border-neutral-700"
	>
		Draft Input (placeholder — page-main-draft で CodeMirror 6 を組む)
	</div>
</section>
