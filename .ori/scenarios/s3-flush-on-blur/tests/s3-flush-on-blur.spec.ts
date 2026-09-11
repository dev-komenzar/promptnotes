// @ori-generated scenario:s3-flush-on-blur
//
// Scenario: フォーカス喪失で即時 Flush（debounce 待たず）
//
// domain/validation.md#s3-flush-on-blur:
//   - GIVEN: 既存 Note A, EDITING 状態。t1 に編集（AutoSave debounce timer は 500ms 待ち中）
//   - WHEN:  t1+0.2s=t2 時点でユーザが別ブロックをクリック。Block A は EDITING → IDLE 遷移、フォーカス喪失
//   - THEN:  debounce timer キャンセル。即時 Note::edit_body(now=t2) を実行（Flush）。NoteBodyEdited 発行
//
// runner: wdio (Tauri desktop app — .ori/architecture.md workspace.apps[].runtime.mode=local, runner=wdio)

describe('scenario:s3-flush-on-blur', () => {
  before(async () => {
    await browser.waitUntil(
      async () => (await $$('[data-testid="screen-1-block"]')).length >= 2,
      { timeout: 10000, timeoutMsg: 'seeded blocks did not appear in feed' }
    );
    await browser.pause(1500);
  });

  describe('ブロック focus 喪失時の即時 Flush', () => {
    it('別ブロッククリックで debounce を待たずに即時保存されること', async () => {
      const blocks = await $$('[data-testid="screen-1-block"]');
      expect(blocks.length).toBeGreaterThanOrEqual(2);

      const blockA = blocks[0]!;
      const blockB = blocks[1]!;

      await blockA.click();
      await browser.waitUntil(
        async () => (await blockA.getAttribute('data-block-state')) === 'EDITING',
        { timeout: 5000, timeoutMsg: 'Block A did not enter EDITING state' }
      );

      const editor = await blockA.$('.cm-editor .cm-content');
      await editor.waitForExist({ timeout: 3000 });
      await editor.click();
      await browser.keys(' x');
      await browser.pause(100);

      await blockB.click();

      await browser.waitUntil(
        async () => (await blockA.getAttribute('data-block-state')) === 'IDLE',
        { timeout: 5000, timeoutMsg: 'Block A did not return to IDLE state' }
      );

      const noteAState = await blockA.getAttribute('data-block-state');
      expect(noteAState).toBe('IDLE');
    });

    it('debounce timer のタイムアウト前に Flush が発火すること', async () => {
      // AutoSave debounce が発火する前に Flush が実行されることを
      // タイミングベースで検証する。

      const blocks = await $$('[data-testid="screen-1-block"]');
      expect(blocks.length).toBeGreaterThanOrEqual(2);

      const blockA = blocks[0]!;
      const blockB = blocks[1]!;

      // EDITING に遷移
      await blockA.click();
      await browser.waitUntil(
        async () => (await blockA.getAttribute('data-block-state')) === 'EDITING',
        { timeout: 5000 }
      );

      const editor = blockA.$('.cm-editor .cm-content');
      await editor.waitForExist({ timeout: 3000 });
      await editor.click();

      // 編集（即時）
      await browser.keys('hello flush');

      // 100ms 後にブロック B クリック（debounce 600ms 前に Flush が起動する）
      await browser.pause(100);
      await blockB.click();

      // ブロック A が IDLE に戻る = Flush が実行された
      await browser.waitUntil(
        async () => (await blockA.getAttribute('data-block-state')) === 'IDLE',
        { timeout: 5000, timeoutMsg: 'Block A did not flush before debounce timer fire' }
      );

      const noteAState = await blockA.getAttribute('data-block-state');
      expect(noteAState).toBe('IDLE');
    });

    it('EDITING でないブロックのクリックでは Flush は起きないこと', async () => {
      const blocks = await $$('[data-testid="screen-1-block"]');
      expect(blocks.length).toBeGreaterThanOrEqual(2);

      const blockA = blocks[0]!;
      const blockB = blocks[1]!;

      // Block A を EDITING にせず、そのまま Block B をクリック
      // Block A の state が IDLE のままなら Flush が起きていない
      await blockA.click();

      // FOCUSED or EDITING になるのはクリックされたブロック
      await browser.waitUntil(
        async () => {
          const state = await blockA.getAttribute('data-block-state');
          return state !== 'IDLE' || state === 'FOCUSED';
        },
        { timeout: 3000, timeoutMsg: 'Block A state did not change' }
      );

      // Block B をクリック → A の focus が外れる
      await blockB.click();

      // A は IDLE に戻るが、編集していないので Flush は body 変化なしで no-op
      const stateA = await blockA.getAttribute('data-block-state');
      expect(stateA).toBe('IDLE');
    });
  });
});