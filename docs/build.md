# Build & Release Strategy

このドキュメントは promptnotes の **ビルド戦略 / signing key 管理 / リリース手順** をまとめたものです。
背景の議論ログは残していないので、判断に齟齬が出た場合はコンテキストとして読み直してください。

---

## 1. 全体方針

| 項目 | 方針 |
|---|---|
| 開発 | Linux (NixOS) メイン機で完結。日常の編集・テスト・build はここで行う |
| Linux release build | Linux メイン機 (NixOS) で native build |
| macOS release build | mac サブ機で build + 自己署名 (codesign のみ) |
| Windows | **初期 MVP から除外**。マシンを保有していないため |
| CI | **補助的にしか使わない**。基本はローカルビルド。GitHub Actions の有料 runner は極力避ける |
| Apple Developer Program | **参加しない**。Developer ID / notarization なし。配布時に Gatekeeper warning が出る前提で運用 |

**判断軸**:

- ローカルビルドの再現性は Nix flake で担保する (NixOS 上で `nix develop`)
- Tauri の cross-platform binary 化は無理にやらない。各 platform の native 環境を使う
- signing key は platform ごとに物理マシンに紐づけるため、nix-sops に集約するうまみは薄い。**Tauri updater keypair のみ nix-sops で共有**
- 年額コスト (Apple Developer Program $99/年) と初回起動時の user friction を天秤にかけて、Developer Program 不参加を選択する

---

## 2. 開発環境

### 2.1 Linux (NixOS) — メイン機

Nix flake でツールチェーンを固定。

```bash
# 初回のみ: direnv を許可
direnv allow

# dev shell に入る (cd で自動 enter、nix develop でも可)
nix develop

# 依存 install
cd apps/promptnotes
bun install

# 開発 server
bun run dev

# Linux release build (Tauri)
bun run tauri build --bundles deb,appimage,rpm

# 統合 build (Nix flake)
nix build                                # 全 platform の binary を build
nix run                                  # build 済み binary を起動
```

### 2.2 macOS — サブ機

mac サブ機には既に **Nix + nix-sops** 環境が構築済み (前提)。
Tauri build に必要な以下を追加で install する。

```bash
# Rust toolchain (rustup)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# bun
curl -fsSL https://bun.sh/install | bash

# Xcode Command Line Tools (codesign に必要)
xcode-select --install
```

```bash
# mac release build
cd apps/promptnotes
bun install
bun run tauri build --bundles app,dmg
```

---

## 3. Signing key 管理

> **用語の整理**: 「Signing key」と一括りにされがちですが、このプロジェクトでは **3 種類の独立した鍵** を使います。
>
> | 鍵 | 用途 | 技術 |
> |---|---|---|
> | Tauri updater keypair | **更新バイナリの検証** (in-app updater) | ed25519 |
> | macOS コード署名証明書 | **app binary 自体**の信頼 (Gatekeeper) | X.509 |
> | Linux GPG key | **deb / rpm パッケージ**の信頼 | OpenPGP |
>
> それぞれ目的・技術・保管場所が違います。以下、混同しないよう鍵の種類を明示して記述します。

### 3.1 全体方針

| key の種類 | 保管場所 | 共有範囲 |
|---|---|---|
| macOS コード署名証明書 | **mac サブ機の Keychain に自己署名で作成** (Developer ID ではない) | mac のみ |
| Linux GPG (deb/rpm 署名) | **NixOS の既存 GPG home に追加** | Linux のみ |
| Tauri updater public key | **repo に平文で commit** (`tauri.conf.json` の `updater.pubkey` 等) | 配布物なので公開 |
| Tauri updater private key | **nix-sops で暗号化して commit** | Linux / mac で共有 |

> **Developer ID / notarization は使わない** ため、証明書は自己署名 (Self-Signed Root) で十分。`pass` 等で同期する secret も減る。

### 3.2 nix-sops を使う理由 (Tauri updater keypair のみ)

- **個人用 secret の既存運用に乗っかれる** (ssh / age key と同じ flow)
- **Linux で生成 → git commit → mac で復号** がコードで残せる
- **release 時しか使わない secret** なので nix-sops の daily ergonomics 不要

platform 別マシンで build する以上、Apple Keychain / GPG はそれぞれのマシンでローカル管理が自然。
唯一 updater keypair だけが「両方のマシンで必要」なので、nix-sops で共有する価値がある。

### 3.3 Tauri updater keypair の運用

#### 生成 (NixOS 上で 1 回だけ)

```bash
# 1. ed25519 keypair を生成
openssl genpkey -algorithm ed25519 -out /tmp/updater.key
openssl pkey -in /tmp/updater.key -pubout -out updater.pub

# 2. private を nix-sops で暗号化 (既存の age key を使う)
mkdir -p secrets/tauri
sops --encrypt --input-type binary --output-type yaml \
  /tmp/updater.key > secrets/tauri/updater.key.sops.yaml

# 3. 平文は完全に削除
shred -u /tmp/updater.key

# 4. public は平文で commit (配布物)
mv updater.pub secrets/tauri/updater.pub
```

`tauri.conf.json` の updater 設定に public key を埋め込む:

```json
{
  "plugins": {
    "updater": {
      "pubkey": "<secrets/tauri/updater.pub の中身>"
    }
  }
}
```

#### mac へ Tauri updater private key を配置

NixOS の `secrets/tauri/updater.key.sops.yaml` を mac の Nix flake 経由で復号し、`~/.config/tauri/updater.key` に配置する。Nix + nix-sops の運用は既存のものを使う (手順は nix-sops の慣習に従う)。

#### build 時に使う

```bash
# Linux release
bun run tauri build \
  --signing-key ~/.config/tauri/updater.key \
  --signing-key-password "$(pass show tauri/updater)"

# mac release (mac サブ機)
bun run tauri build \
  --signing-key ~/.config/tauri/updater.key \
  --signing-key-password "$(pass show tauri/updater)" \
  --bundles app,dmg
```

> `pass` (GPG-based password manager) は mac / Linux 両方で同期可能。updater key の password を別途保管する必要がある。

### 3.4 やってはいけないこと

- **Developer ID を取得しようとしない** (Apple Developer Program に参加しないため)。コード署名は自己署名で運用する
- **updater keypair を platform ごとに生成しない**: Linux release と mac release で署名鍵が変わると検証が壊れる
- **CI runner に personal secret を持ち込まない前提で設計する**: 仮に後述の CI 併用フェーズに移行しても、private secret は GitHub Secrets 等に移し、nix-sops の repository は個人のまま分離する

---

## 4. ローカルビルド手順

### 4.1 Linux build (NixOS メイン機)

```bash
# dev shell に入る
nix develop

cd apps/promptnotes
bun install
bun run tauri build --bundles deb,appimage,rpm
```

成果物: `apps/promptnotes/src-tauri/target/release/bundle/{deb,rpm,appimage}/`

deb/rpm には GPG 署名が必要:

```bash
# GPG key の準備 (初回のみ)
gpg --quick-gen-key "promptnotes release <release@promptnotes.local>"
gpg --export-secret-keys <key-id> > ~/.local/share/promptnotes/release-gpg.asc  # バックアップ
chmod 600 ~/.local/share/promptnotes/release-gpg.asc

# build 時に sign
dpkg-sig --sign builder -k <key-id> *.deb
rpm --addsign *.rpm
```

### 4.2 macOS build + 自己署名 (mac サブ機)

```bash
cd apps/promptnotes
bun install
bun run tauri build \
  --signing-key ~/.config/tauri/updater.key \
  --signing-key-password "$(pass show tauri/updater)" \
  --bundles app,dmg
```

成果物: `apps/promptnotes/src-tauri/target/release/bundle/macos/*.app` と `*.dmg`

#### 自己署名証明書の作成 (初回のみ)

`Keychain Access.app` で:

1. **Keychain Access → Certificate Assistant → Create a Certificate**
2. Identity Type: **Self Signed Root**
3. Certificate Type: **Code Signing**
4. Name: `promptnotes-local` (任意)
5. 「Let me override defaults」をチェック → 続きは既定のままで OK

CLI でやる場合は:

```bash
security create-certificate \
  -c "promptnotes-local" \
  -t ctsm \
  -k "$(security default-keychain | xargs)" \
  /tmp/promptnotes-local.cert
```

#### codesign

```bash
codesign --force --deep --options runtime \
  --sign "promptnotes-local" \
  target/release/bundle/macos/promptnotes.app

# 確認
codesign --verify --deep --strict --verbose=2 \
  target/release/bundle/macos/promptnotes.app
```

> `Developer ID Application: ...` ではなく、自己署名証明書 (例: `promptnotes-local`) で署名する。`--timestamp` は付けない (TSA サーバへの問い合わせに Developer ID が要求されるため)。

#### notarization は行わない

notarization には Apple Developer Program の team ID が必須なので、**行わない**。
代わりに配布先で初回起動時に Gatekeeper warning が出ることを許容する。
ユーザ向け回避手順は [README.md](../README.md) の「macOS での起動」セクションに記載する (後述の配布戦略を参照)。

### 4.3 release の一連の流れ

1. version bump (`apps/promptnotes/package.json` と `apps/promptnotes/src-tauri/Cargo.toml`)
2. `git tag vX.Y.Z` & push
3. Linux build → 成果物を release draft に upload
4. mac build + 自己署名 → 成果物を release draft に upload
5. release notes に **macOS 初回起動時の Gatekeeper 回避手順** を必ず記載
6. publish

Tauri の updater は GitHub Releases を配信元にする想定。
updater の署名検証は Developer ID / notarization と独立なので、Apple Developer Program 不参加でも Tauri updater は問題なく機能する。

#### macOS 配布先の Gatekeeper 回避手順 (README に貼るテンプレ)

```markdown
### macOS での起動

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
```

---

## 5. 補足

### 5.1 Windows を MVP から除外する理由

- 物理マシンを保有していない
- Wine + `osslsigncode` でクロス sign は可能だが、本物の EV cert フロー (SmartScreen 即時信頼等) は結局 Windows 環境が必要
- 個人開発のリソース制約上、優先度は mac / Linux に置く

### 5.2 CI 移行の判断基準

以下のいずれかに該当したら CI の併用を検討する:

- mac サブ機の電源を入れる頻度が月に数回以下になり、release のたびに起動が面倒
- 共同開発者が増え、複数マシンで同時 release 検証が必要になった
- Windows 対応を再開する

その場合は:

- macOS build は GitHub Actions の **有料 mac runner** ($0.08/min) をスポット利用
- private secret は GitHub Actions Secrets に移し、nix-sops repo は個人用として分離維持
- Linux build は自前 runner (NixOS) または GitHub-hosted ubuntu で

### 5.3 配布戦略 (notarization なし)

notarization しないことを前提に、配布経路ごとの得失:

| 配布経路 | ユーザの手間 | 構築コスト | 備考 |
|---|---|---|---|
| **GitHub Releases 直配布** | 初回起動時に右クリックで開く | ゼロ | 最もシンプル。Tauri updater の配信元としても使える |
| **Homebrew Cask** | `brew install --cask promptnotes` のみ | Cask formula 作成が必要 | README に Gatekeeper 回避手順を併記する必要あり |
| **個人 Web サイト** | 右クリックで開く | サイト運用 | 個人開発では過剰 |

**推奨**: GitHub Releases を主軸、必要に応じて Homebrew Cask を後から追加。

### 5.4 関連ドキュメント

- [README.md](../README.md) — プロジェクト概要・quick start
- [.ori/architecture.md](../.ori/architecture.md) — DDD / VSA 設計の single source of truth
- [idea.md](../idea.md) — 初期構想 (frozen)