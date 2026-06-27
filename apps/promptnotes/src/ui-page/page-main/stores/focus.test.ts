import { describe, expect, it } from 'vitest';
import { createFocusStore } from './focus.svelte';

describe('page:page-main focus store', () => {
	it('spec#tp-block-state-machine — 初期状態は IDLE / activeId=null', () => {
		const store = createFocusStore();
		expect(store.activeId).toBeNull();
		expect(store.activeState).toBe('IDLE');
		expect(store.stateOf('any')).toBe('IDLE');
	});

	it('spec#tp-block-state-machine — IDLE → FOCUSED: navigate("next") は最初の Block を FOCUSED に', () => {
		const store = createFocusStore();
		store.navigate('next', ['a', 'b']);
		expect(store.activeId).toBe('a');
		expect(store.activeState).toBe('FOCUSED');
		expect(store.stateOf('a')).toBe('FOCUSED');
		expect(store.stateOf('b')).toBe('IDLE');
	});

	it('spec#tp-block-state-machine — FOCUSED → FOCUSED (別): navigate で隣へ移る', () => {
		const store = createFocusStore();
		store.focus('a');
		store.navigate('next', ['a', 'b', 'c']);
		expect(store.activeId).toBe('b');
		expect(store.activeState).toBe('FOCUSED');
		store.navigate('prev', ['a', 'b', 'c']);
		expect(store.activeId).toBe('a');
	});

	it('spec#tp-block-state-machine — FOCUSED → EDITING: enter()', () => {
		const store = createFocusStore();
		store.focus('a');
		store.enter();
		expect(store.activeState).toBe('EDITING');
		expect(store.stateOf('a')).toBe('EDITING');
	});

	it('spec#tp-block-state-machine — IDLE → EDITING: edit() (click)', () => {
		const store = createFocusStore();
		store.edit('a');
		expect(store.activeId).toBe('a');
		expect(store.activeState).toBe('EDITING');
	});

	it('spec#tp-block-state-machine — EDITING → 別 EDITING: 前 active は IDLE になる', () => {
		const store = createFocusStore();
		store.edit('a');
		store.edit('b');
		expect(store.activeId).toBe('b');
		expect(store.activeState).toBe('EDITING');
		expect(store.stateOf('a')).toBe('IDLE');
		expect(store.stateOf('b')).toBe('EDITING');
	});

	it('spec#tp-block-state-machine — FOCUSED → IDLE: escape()', () => {
		const store = createFocusStore();
		store.focus('a');
		store.escape();
		expect(store.activeId).toBeNull();
		expect(store.activeState).toBe('IDLE');
	});

	it('spec#tp-block-state-machine — EDITING → FOCUSED: escape()', () => {
		const store = createFocusStore();
		store.edit('a');
		store.escape();
		expect(store.activeId).toBe('a');
		expect(store.activeState).toBe('FOCUSED');
	});

	it('spec#I-PM10 — EDITING 中の navigate は no-op', () => {
		const store = createFocusStore();
		store.edit('a');
		store.navigate('next', ['a', 'b', 'c']);
		expect(store.activeId).toBe('a');
		expect(store.activeState).toBe('EDITING');
		store.navigate('prev', ['a', 'b', 'c']);
		expect(store.activeId).toBe('a');
		expect(store.activeState).toBe('EDITING');
	});

	it('navigate は最初/最後で頭打ち (循環しない)', () => {
		const store = createFocusStore();
		store.focus('a');
		store.navigate('prev', ['a', 'b', 'c']);
		expect(store.activeId).toBe('a');
		store.focus('c');
		store.navigate('next', ['a', 'b', 'c']);
		expect(store.activeId).toBe('c');
	});

	it('navigate("prev") on IDLE は最後の Block を FOCUSED に', () => {
		const store = createFocusStore();
		store.navigate('prev', ['a', 'b', 'c']);
		expect(store.activeId).toBe('c');
		expect(store.activeState).toBe('FOCUSED');
	});

	it('clear() は IDLE / activeId=null に戻す', () => {
		const store = createFocusStore();
		store.edit('a');
		store.clear();
		expect(store.activeId).toBeNull();
		expect(store.activeState).toBe('IDLE');
	});

	it('navigate(空 ids) は no-op', () => {
		const store = createFocusStore();
		store.focus('a');
		store.navigate('next', []);
		expect(store.activeId).toBe('a');
	});
});
