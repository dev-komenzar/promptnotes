/**
 * page-main Block state machine (screen-1.md#cross-block-state, spec#tp-block-state-machine).
 *
 * 全 Block の中で同時に EDITING / FOCUSED な Block は高々 1 つ (I-PM10)。
 * 他の Block は IDLE。activeId が変わった瞬間に前 active Block は IDLE へ落ちる。
 *
 * 遷移:
 * - IDLE → FOCUSED: `↑` / `↓` (`navigate`)
 * - IDLE → EDITING: click (`edit`)
 * - FOCUSED → EDITING: `Enter` (`enter`)
 * - FOCUSED → FOCUSED (別): `↑` / `↓` (`navigate`)
 * - FOCUSED → IDLE: `Esc` (`escape`)
 * - EDITING → FOCUSED: `Esc` (`escape`)
 * - EDITING → 別 EDITING: click on other (`edit`)
 * - EDITING 中の `↑/↓` は no-op (`navigate` 早期 return)
 */
export type BlockState = 'IDLE' | 'FOCUSED' | 'EDITING';

export type FocusStore = ReturnType<typeof createFocusStore>;

export function createFocusStore() {
	let activeId = $state<string | null>(null);
	let activeState = $state<BlockState>('IDLE');

	function stateOf(id: string): BlockState {
		return activeId === id ? activeState : 'IDLE';
	}

	function focus(id: string): void {
		activeId = id;
		activeState = 'FOCUSED';
	}

	function edit(id: string): void {
		activeId = id;
		activeState = 'EDITING';
	}

	function escape(): void {
		if (activeState === 'EDITING') {
			activeState = 'FOCUSED';
		} else if (activeState === 'FOCUSED') {
			activeId = null;
			activeState = 'IDLE';
		}
	}

	function enter(): void {
		if (activeState === 'FOCUSED' && activeId !== null) {
			activeState = 'EDITING';
		}
	}

	function navigate(direction: 'prev' | 'next', visibleIds: readonly string[]): void {
		if (activeState === 'EDITING') return;
		if (visibleIds.length === 0) return;
		const idx = activeId !== null ? visibleIds.indexOf(activeId) : -1;
		let nextIdx: number;
		if (idx === -1) {
			nextIdx = direction === 'next' ? 0 : visibleIds.length - 1;
		} else if (direction === 'next') {
			nextIdx = Math.min(idx + 1, visibleIds.length - 1);
		} else {
			nextIdx = Math.max(idx - 1, 0);
		}
		activeId = visibleIds[nextIdx];
		activeState = 'FOCUSED';
	}

	function clear(): void {
		activeId = null;
		activeState = 'IDLE';
	}

	return {
		get activeId() {
			return activeId;
		},
		get activeState() {
			return activeState;
		},
		stateOf,
		focus,
		edit,
		escape,
		enter,
		navigate,
		clear
	};
}

export const focusStore = createFocusStore();
