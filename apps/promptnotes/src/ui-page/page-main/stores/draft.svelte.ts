import { createNote, type CreateNoteOutcome } from '$lib/note-capture/slices/create-note';

export type DraftStoreDeps = {
	create?: typeof createNote;
};

export type DraftSubmitOutcome =
	| { outcome: 'created'; id: string; created_at: string }
	| { outcome: 'no_op' };

export type TagAddOutcome =
	| { outcome: 'added' }
	| { outcome: 'invalid'; reason: string };

export type DraftStore = ReturnType<typeof createDraftStore>;

/** Tag validation matching I-N6 from Tag domain: no whitespace, commas, brackets. */
function validateTag(raw: string): string | null {
	const trimmed = raw.trim().toLowerCase();
	if (trimmed === '') return null;
	if (/[\s,[\]()]/.test(trimmed)) return null;
	return trimmed;
}

export function createDraftStore(deps: DraftStoreDeps = {}) {
	const create = deps.create ?? createNote;

	let body = $state('');
	let tags = $state<string[]>([]);
	let submitting = $state(false);

	function setBody(next: string): void {
		body = next;
	}

	function addTag(raw: string): TagAddOutcome {
		const validated = validateTag(raw);
		if (validated === null) {
			return { outcome: 'invalid', reason: 'タグに使えない文字（カンマ・ブラケット・空白）が含まれています' };
		}
		if (!tags.includes(validated)) {
			tags = [...tags, validated];
		}
		return { outcome: 'added' };
	}

	function removeTag(tag: string): void {
		tags = tags.filter((t) => t !== tag);
	}

	function clear(): void {
		body = '';
		tags = [];
	}

	async function submit(): Promise<DraftSubmitOutcome> {
		if (submitting) return { outcome: 'no_op' };
		submitting = true;
		try {
			const result: CreateNoteOutcome = await create(body, tags);
			if (result.outcome === 'created') {
				clear();
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
		get tags() {
			return tags;
		},
		get submitting() {
			return submitting;
		},
		setBody,
		addTag,
		removeTag,
		clear,
		submit
	};
}

export const draftStore = createDraftStore();
