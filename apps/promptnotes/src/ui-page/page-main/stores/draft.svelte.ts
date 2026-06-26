import { createNote, type CreateNoteOutcome } from '$lib/note-capture/slices/create-note';

export type DraftStoreDeps = {
	create?: typeof createNote;
};

export type DraftSubmitOutcome =
	| { outcome: 'created'; id: string; created_at: string }
	| { outcome: 'no_op' };

export type DraftStore = ReturnType<typeof createDraftStore>;

export function createDraftStore(deps: DraftStoreDeps = {}) {
	const create = deps.create ?? createNote;

	let body = $state('');
	let submitting = $state(false);

	function setBody(next: string): void {
		body = next;
	}

	function clear(): void {
		body = '';
	}

	async function submit(): Promise<DraftSubmitOutcome> {
		if (submitting) return { outcome: 'no_op' };
		submitting = true;
		try {
			const result: CreateNoteOutcome = await create(body, []);
			if (result.outcome === 'created') {
				body = '';
			}
			return result;
		} finally {
			submitting = false;
		}
	}

	return {
		get body() {
			return body;
		},
		get submitting() {
			return submitting;
		},
		setBody,
		clear,
		submit
	};
}

export const draftStore = createDraftStore();
