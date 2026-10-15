import { describe, expect, it, vi } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import type { NoteFileDeletedExternallyPayload } from '../store.svelte';

const { default: WidgetExternalDeleteNotice } =
	await import('../WidgetExternalDeleteNotice.svelte');

const NOTE_ID = '20260620120000';
const ROOT = 'widget.widget-external-delete-notice.root';

function makePayload(): NoteFileDeletedExternallyPayload {
	return {
		note_id: NOTE_ID,
		note_title: `${NOTE_ID}.md`,
		note_body: 'hello local',
		file_path: `/tmp/notes/${NOTE_ID}.md`,
		detected_at: '2026-06-20T12:00:00Z'
	};
}

async function renderOpenNotice() {
	let emit: ((payload: NoteFileDeletedExternallyPayload) => void) | null = null;
	const onSaveAsNew = vi.fn();
	const onDiscard = vi.fn();
	render(WidgetExternalDeleteNotice, {
		deps: {
			subscribeFn: async (handler) => {
				emit = handler;
				return () => {};
			},
			currentNoteId: () => NOTE_ID,
			onSaveAsNew,
			onDiscard
		}
	});
	// store.start() が subscribe を張るのを待ってから削除通知 payload を届ける
	await vi.waitFor(() => expect(emit).not.toBeNull());
	emit!(makePayload());
	await expect.element(page.getByTestId(ROOT)).toBeInTheDocument();
	const dialog = document.querySelector<HTMLDialogElement>(`[data-testid="${ROOT}"]`)!;
	await vi.waitFor(() => expect(dialog.open).toBe(true));
	return { dialog, onSaveAsNew, onDiscard };
}

describe('widget:widget-external-delete-notice component', () => {
	it('spec.md#tp-del-no-implicit-close — cancel event では閉じず、解決 callback も呼ばれない (I-DEL6)', async () => {
		const { dialog, onSaveAsNew, onDiscard } = await renderOpenNotice();

		const cancel = new Event('cancel', { cancelable: true });
		dialog.dispatchEvent(cancel);

		expect(cancel.defaultPrevented).toBe(true);
		expect(dialog.open).toBe(true);
		await expect.element(page.getByTestId(ROOT)).toBeVisible();
		expect(onSaveAsNew).not.toHaveBeenCalled();
		expect(onDiscard).not.toHaveBeenCalled();
	});

	it('spec.md#tp-del-no-implicit-close — Escape キーでは閉じず、解決 callback も呼ばれない (I-DEL6)', async () => {
		const { dialog, onSaveAsNew, onDiscard } = await renderOpenNotice();

		await userEvent.keyboard('{Escape}');

		expect(dialog.open).toBe(true);
		await expect.element(page.getByTestId(ROOT)).toBeVisible();
		expect(onSaveAsNew).not.toHaveBeenCalled();
		expect(onDiscard).not.toHaveBeenCalled();
	});
});
