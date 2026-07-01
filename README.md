# promptnotes

AI プロンプトを書き溜め、すぐコピーして使うためのデスクトップノートアプリ。

> Claude Code や Codex で Enter を押して意図せず送信してしまったことはありませんか？
> Prompt Notes なら、安心して Enter を押せます。

シングルペイン・CodeMirror 常時表示・markdown を `.md` ファイルとしてローカル保存。
Tauri v2 / SvelteKit / Rust 製。

> 構想の詳細は [idea.md](./idea.md)、ドメインモデルは [.ori/domain/](./.ori/domain/) を参照。

---

## 対応プラットフォーム

| platform | 対応状況 | ビルド環境 |
|---|---|---|
| Linux (deb/rpm/AppImage) | 対応 | NixOS メイン機 |
| macOS (.app/.dmg) | 対応 | mac サブ機 |
| Windows | **初期 MVP から除外** | — |

> **macOS について**: Apple Developer Program に登録していないため、公証 (notarization) を行わず自己署名で配布します。配布経路や Gatekeeper 回避手順は [docs/build.md](./docs/build.md) を参照。

詳細な戦略は [docs/build.md](./docs/build.md) を参照。

---

## macOS での起動

promptnotes は Apple Developer Program に登録していないため、公証 (notarization) されていません。
初回起動時に Gatekeeper の警告が出ます。以下のいずれかで回避してください:

**方法 A: 右クリックで開く (1 回だけ)**
1. Finder で `promptnotes.app` を右クリック (control + クリック)
2. 「開く」を選択
3. 再度警告が出るが、もう一度「開く」をクリック

**方法 B: ターミナルから属性を解除**

```bash
xattr -dr com.apple.quarantine /Applications/promptnotes.app
```

以降は普通に起動できます。

---

## クイックスタート (開発)

Nix flake でツールチェーンを固定。

```bash
# 1. direnv を許可 (Nix 開発環境に入る)
direnv allow

# 2. 依存 install
cd apps/promptnotes
bun install

# 3. 開発 server 起動
bun run dev
```

ビルド・テスト・リリース手順は [docs/build.md](./docs/build.md)。

---

## 技術スタック

- **Desktop framework**: Tauri v2
- **Frontend**: SvelteKit (static adapter) + Vite + CodeMirror 6
- **Backend**: Rust (Tauri command)
- **テスト**: Vitest (unit + component) / `cargo test` (Rust)
- **パッケージマネージャ**: bun
- **ツールチェーン固定**: Nix flake

---

## プロジェクト構成

```
.
├── apps/promptnotes/          # SvelteKit + Tauri app
│   ├── src/                   # frontend
│   └── src-tauri/             # Rust backend
├── .ori/                      # DDD distill 文書 (single source of truth)
├── docs/                      # 補足ドキュメント
│   └── build.md               # ビルド・リリース戦略
├── flake.nix                  # Nix devShell + package
└── idea.md                    # 初期構想 (frozen)
```

---

## License

未設定 (個人開発)。