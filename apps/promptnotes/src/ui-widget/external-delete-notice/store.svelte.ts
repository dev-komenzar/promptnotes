/**
 * widget-external-delete-notice の削除通知 + 解決ロジックを保持する store (scenario S20)。
 *
 * - I-DEL1: page-main mount 時に subscribe を 1 回張り、unmount で解除
 * - I-DEL2: `NoteFileDeletedExternally` 相当の削除 payload 未受信時は payload=null（silent）
 * - I-DEL3: 同一 note_id の重複通知は表示中は無視（debounce）
 * - I-DEL4: SaveAsNew → onSaveAsNew callback 呼出 → hidden
 * - I-DEL5: Discard → onDiscard callback 呼出 → hidden
 *
 * domain event `NoteFileDeletedExternally` は frontend に露出しないため、payload は
 * page-main の bridge が `notes-changed` + `list_notes` の存在差分から構成する。
 */

export type NoteFileDeletedExternallyPayload = {
	note_id: string;
	note_title: string;
	/** 現在の編集中 body（ローカル snapshot） */
	note_body: string;
	file_path: string;
	detected_at: string;
};

export type DeleteNoticeSubscribeFn = (
	handler: (payload: NoteFileDeletedExternallyPayload) => void
) => Promise<() => void>;

export type ExternalDeleteNoticeStoreDeps = {
	subscribeFn?: DeleteNoticeSubscribeFn;
	currentNoteId?: () => string | null;
	onSaveAsNew?: (payload: NoteFileDeletedExternallyPayload) => void | Promise<void>;
	onDiscard?: (payload: NoteFileDeletedExternallyPayload) => void | Promise<void>;
};

export type ExternalDeleteNoticeStore = ReturnType<typeof createExternalDeleteNoticeStore>;

const defaultSubscribeFn: DeleteNoticeSubscribeFn = async () => {
	// Default listener is a no-op; production wiring injects a real subscribeFn
	// from external-change-bridge.deleteDeps().
	return () => {};
};

export function createExternalDeleteNoticeStore(deps: ExternalDeleteNoticeStoreDeps = {}) {
	const subscribeFn = deps.subscribeFn ?? defaultSubscribeFn;
	const currentNoteId = deps.currentNoteId ?? (() => null);
	const onSaveAsNew = deps.onSaveAsNew ?? (() => {});
	const onDiscard = deps.onDiscard ?? (() => {});

	let payload = $state<NoteFileDeletedExternallyPayload | null>(null);
	let state = $state<'hidden' | 'notice'>('hidden');
	let unsubscribe: (() => void) | null = null;

	async function start(): Promise<void> {
		if (unsubscribe) return;
		try {
			unsubscribe = await subscribeFn((next) => {
				// silent if not editing this note
				if (currentNoteId() !== next.note_id) return;
				// I-DEL3: ignore duplicate while notice is open
				if (payload !== null && payload.note_id === next.note_id) return;
				payload = next;
				state = 'notice';
			});
		} catch {
			// silent failure — subscribe 失敗時は何も表示しない
		}
	}

	function stop(): void {
		unsubscribe?.();
		unsubscribe = null;
	}

	async function saveAsNew(): Promise<void> {
		const current = payload;
		if (current !== null) await onSaveAsNew(current);
		payload = null;
		state = 'hidden';
	}

	async function discard(): Promise<void> {
		const current = payload;
		if (current !== null) await onDiscard(current);
		payload = null;
		state = 'hidden';
	}

	return {
		get payload() {
			return payload;
		},
		get state() {
			return state;
		},
		start,
		stop,
		saveAsNew,
		discard
	};
}
