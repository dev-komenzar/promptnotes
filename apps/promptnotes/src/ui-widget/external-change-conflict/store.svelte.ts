/**
 * widget-external-change-conflict の競合検出 + 解決ロジックを保持する store。
 *
 * - I-WC1: page-main mount 時に subscribe を 1 回張り、unmount で解除
 * - I-WC2: NoteFileModifiedExternally 未受信 / isStale false の場合は conflictPayload=null（silent）
 * - I-WC3: 同一 note_id の重複 event はダイアログ表示中は無視（debounce）
 * - I-WC5: ApplyExternal 選択 + apply() → onApplyExternal callback 呼出 → hidden
 * - I-WC6: KeepEditing / cancel() → callback 呼ばず hidden。次回 save で上書き
 * - I-WC7: ダイアログ表示中は Editor キー入力をブロック（呼び出し元の責務）
 */

export type ConflictResolution = 'ApplyExternal' | 'KeepEditing';

export type NoteFileModifiedExternallyPayload = {
	note_id: string;
	disk_body_hash: string;
	note_title: string;
	note_body: string;
	file_path: string;
	detected_at: string;
};

export type ConflictSubscribeFn = (
	handler: (payload: NoteFileModifiedExternallyPayload) => void
) => Promise<() => void>;

export type ConflictDialogStoreDeps = {
	subscribeFn?: ConflictSubscribeFn;
	isStaleFn?: (localHash: string, diskHash: string) => boolean;
	onApplyExternal?: (payload: NoteFileModifiedExternallyPayload) => void | Promise<void>;
	currentNoteId?: () => string | null;
	currentBodyHash?: () => string | null;
};

export type ConflictDialogStore = ReturnType<typeof createConflictDialogStore>;

const defaultSubscribeFn: ConflictSubscribeFn = async () => {
	// OQ-WC1: Real event bridge (Rust → TS) is not yet wired.
	// Default listener is a no-op. Production wiring will inject a real subscribeFn.
	return () => {};
};

export function createConflictDialogStore(deps: ConflictDialogStoreDeps = {}) {
	const subscribeFn = deps.subscribeFn ?? defaultSubscribeFn;
	const isStaleFn = deps.isStaleFn ?? ((a, b) => a !== b);
	const onApplyExternal = deps.onApplyExternal ?? (() => {});
	const currentNoteId = deps.currentNoteId ?? (() => null);
	const currentBodyHash = deps.currentBodyHash ?? (() => null);

	let conflictPayload = $state<NoteFileModifiedExternallyPayload | null>(null);
	let resolution = $state<ConflictResolution>('KeepEditing');
	let state = $state<'hidden' | 'compare'>('hidden');
	let unsubscribe: (() => void) | null = null;

	async function start(): Promise<void> {
		if (unsubscribe) return;
		try {
			unsubscribe = await subscribeFn((payload) => {
				// I-WC2: silent if not editing this note or not stale
				const editingNoteId = currentNoteId();
				if (editingNoteId !== payload.note_id) return;

				const localHash = currentBodyHash();
				if (localHash === null) return;

				if (!isStaleFn(localHash, payload.disk_body_hash)) return;

				// I-WC3: debounce — ignore duplicate while dialog is open
				if (conflictPayload !== null && conflictPayload.note_id === payload.note_id) return;

				conflictPayload = payload;
				state = 'compare';
			});
		} catch {
			// I-WC2: silent failure — subscribe 失敗時は何も表示しない
		}
	}

	function stop(): void {
		unsubscribe?.();
		unsubscribe = null;
	}

	function selectResolution(r: ConflictResolution): void {
		resolution = r;
	}

	async function apply(): Promise<void> {
		// I-WC5: ApplyExternal → callback, then hidden
		// I-WC6: KeepEditing → hidden (no callback)
		if (resolution === 'ApplyExternal' && conflictPayload) {
			await onApplyExternal(conflictPayload);
		}
		conflictPayload = null;
		resolution = 'KeepEditing';
		state = 'hidden';
	}

	function cancel(): void {
		// I-WC6: Cancel is equivalent to KeepEditing
		conflictPayload = null;
		resolution = 'KeepEditing';
		state = 'hidden';
	}

	return {
		get conflictPayload() {
			return conflictPayload;
		},
		get resolution() {
			return resolution;
		},
		get state() {
			return state;
		},
		start,
		stop,
		selectResolution,
		apply,
		cancel
	};
}
