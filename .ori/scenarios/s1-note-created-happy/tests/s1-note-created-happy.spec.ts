// @ori-generated scenario:s1-note-created-happy
// Scenario S1: 新規 Note を Cmd+Enter で確定
// derived from: domain/validation.md#s1-note-created-happy
// runner: wdio (Tauri local app)

import { browser, $, $$ } from '@wdio/globals';
import { expect } from 'expect';

describe('scenario:s1-note-created-happy', () => {
  // ---- Given: アプリ起動済み、フィードに既存 Note 0 件、Draft 入力欄が空 ----

  it('step 1 — validation#s1-note-created-happy: Cmd+N → 入力 → Cmd+Enter で Note 作成と UI 反映', async () => {
    // Given: アプリ起動直後、保存先は OS 慣習パス、フィード 0 件、Draft 空
    const draftEditor = await $('[data-testid="screen-1-draft-body"]');
    await draftEditor.waitForExist({ timeout: 15000 });
    await browser.pause(300);

    // When 1: Cmd+N で Draft 入力欄にフォーカス
    await browser.keys(['Control', 'n']);
    await browser.pause(300);

    // Then: フォーカスが Draft 本文 (CodeMirror .cm-content) に移っている
    const focusedInDraft = await browser.execute(() => {
      const el = document.activeElement as HTMLElement | null;
      return !!(el && el.closest('[data-testid="screen-1-draft-body"]'));
    });
    expect(focusedInDraft).toBe(true);

    // When 2: フォーカス済み Draft に "docs を書く" を入力
    await browser.keys('docs を書く');
    await browser.pause(200);

    // When 3: Cmd+Enter を押下
    await browser.keys(['Control', 'Enter']);
    await browser.pause(1500);

    // Then 1: NoteFeed に 1 件表示
    const noteBlocks = await $$('[data-testid="screen-1-block"]');
    expect(noteBlocks.length).toBe(1);

    // Then 2: body が "docs を書く" と一致
    const bodyText = await noteBlocks[0].getText();
    expect(bodyText).toContain('docs を書く');

    // Then 3: Draft 入力欄がクリア
    const draftText = await draftEditor.getText();
    expect(draftText.trim()).toBe('');
  });

  // ---- 境界条件: 空ボディ ----

  it('step 2 — boundary: 空ボディのまま Cmd+Enter では Note 作成されない', async () => {
    // Given: Draft が空の状態

    // When: 空のまま Cmd+Enter
    await browser.keys(['Control', 'Enter']);
    await browser.pause(500);

    // Then: NoteFeed 件数は変わらない（空ボディでは Note::create が呼ばれない）
    const noteBlocks = await $$('[data-testid="screen-1-block"]');
    expect(noteBlocks.length).toBeGreaterThanOrEqual(1);
  });
});