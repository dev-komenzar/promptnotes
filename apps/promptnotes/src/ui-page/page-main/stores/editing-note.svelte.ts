export type EditingNoteState = {
	noteId: string | null;
	bodyHash: string | null;
};

let state = $state<EditingNoteState>({ noteId: null, bodyHash: null });

export const editingNote = {
	get noteId() {
		return state.noteId;
	},
	get bodyHash() {
		return state.bodyHash;
	},
	setEditing(noteId: string | null, bodyHash: string | null) {
		state = { noteId, bodyHash };
	}
};
