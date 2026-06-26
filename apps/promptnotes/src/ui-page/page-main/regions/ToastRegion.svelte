<script lang="ts">
	// page-main-shell では中身は placeholder のみ。
	// 削除 Toast の縦パイル + restore-deleted-note 連携は page-main-toast sub-task (ori-054.5) で実装する。
</script>

<aside
	data-testid="region-toast"
	aria-label="削除 Toast スタック"
	aria-live="polite"
	class="pointer-events-none fixed inset-x-0 bottom-4 z-20 flex flex-col items-center gap-2"
></aside>
