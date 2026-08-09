# Build & Release Strategy

このドキュメントは promptnotes の **ビルド戦略 / updater key 管理 / リリース手順** をまとめたものです。
背景の議論ログは残していないので、判断に齟齬が出た場合はコンテキストとして読み直してください。

---

## 1. 全体方針

| 項目 | 方針 |
|---|---|
| 開発 | Linux (NixOS) メイン機で完結。日常の編集・テスト・build はここで行う |
| Linux release build | Linux メイン機 (NixOS) で native build |
| macOS release build | mac サブ機で build |
| Windows | **初期 MVP から除外**。マシンを保有していないため |
| Linux 配布形式 | **AppImage, .deb, .rpm, Nix flake** の 4 形式 (決定: `ori-wuxk`)。Flatpak/Snap はコミュニティ要望があれば追加検討 |
| CI | Linux は GitHub Actions (ubuntu-22.04) がプライマリ。macOS はローカルビルド。有料 runner は極力避ける |
| Apple Developer Program | **参加しない**。Developer ID / notarization なし。配布時に Gatekeeper warning が出る前提で運用 |

**判断軸**:

- ローカルビルドの再現性は Nix flake で担保する (NixOS 上で `nix develop`)
- Tauri の cross-platform binary 化は無理にやらない。各 platform の native 環境を使う
- コード署名は行わないため、platform 別の署名鍵運用は不要。**Tauri updater keypair のみ nix-sops で共有**
- Tauri updater は Linux 全形式 (AppImage, .deb, .rpm) に対応 (v2.10.0+)。.deb/.rpm は root 権限が必要な点に注意
- Nix flake は開発環境に加えて配布形式としても機能する (`.deb/.rpm` に依存しない独立経路)
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

# 統合 build (Nix flake — dev shell + 配布パッケージ)
nix build                                # Nix パッケージとして build
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

# Xcode Command Line Tools (Tauri build に必要)
xcode-select --install
```

```bash
# mac release build
cd apps/promptnotes
bun install
bun run tauri build --bundles dmg
```

---

## 3. Tauri updater keypair 管理

> **用語の整理**: 「Signing key」と一括りにされがちですが、このプロジェクトで使うのは **Tauri updater keypair のみ** です。
>
> | 鍵 | 用途 | 技術 |
> |---|---|---|
> | Tauri updater keypair | **更新バイナリの検証** (in-app updater) | ed25519 |
>
> macOS のコード署名証明書 (X.509) や Linux の GPG 鍵 (OpenPGP) による **バイナリ / パッケージ署名は行いません**。バイナリは無署名で配布し、Gatekeeper / パッケージマネージャの警告はユーザ側の手順で回避します (5.3 参照)。以下は updater keypair の運用のみを記述します。

### 3.1 全体方針

| key の種類 | 保管場所 | 共有範囲 |
|---|---|---|
| Tauri updater public key | **repo に平文で commit** (`tauri.conf.json` の `updater.pubkey`) | 配布物なので公開 |
| Tauri updater private key | **nix-sops で暗号化して commit** (`secrets/tauri/updater.key.sops.yaml`) | NixOS / macOS で共有 |

鍵の生成には Tauri 組み込みの `cargo tauri signer generate` を使う（minisign 形式で出力されるため、Tauri の updater が期待する形式と完全互換）。

> **Developer ID / notarization は使わない**。さらに **macOS のコード署名 / Linux の GPG パッケージ署名も行わない**。バイナリは無署名で配布し、Gatekeeper / パッケージマネージャの警告はユーザ側の手順で回避します (5.3 参照)。

### 3.2 nix-sops を使う理由

- **個人用 secret の既存運用に乗っかれる** (ssh / age key と同じ flow)
- **NixOS で生成 → git commit → macOS で復号** がコードで残せる
- **release 時しか使わない secret** なので nix-sops の daily ergonomics 不要
- **鍵のバックアップを兼ねる**: nix-sops で暗号化して git 管理すれば、秘密鍵の紛失リスクを回避できる（updater 秘密鍵を失うと、以後のアップデート発行が永久に不可能になる）

platform 別マシンで build するが、コード署名は行わないため Apple Keychain / GPG の運用は不要。
唯一 updater keypair だけが「両方のマシンで必要」なので、nix-sops で共有する価値がある。

### 3.3 Tauri updater keypair の運用

#### 生成 (NixOS 上で 1 回だけ)

```bash
# 1. minisign keypair を生成（パスワードをつけることを推奨）
cargo tauri signer generate -w ~/.tauri/promptnotes.key

# 公開鍵は ~/.tauri/promptnotes.key.pub に生成される。
# 秘密鍵は ~/.tauri/promptnotes.key に生成される（minisign 形式）。

# 2. 公開鍵を tauri.conf.json に埋め込む
# ~/.tauri/promptnotes.key.pub の中身をそのままコピー
```

`tauri.conf.json` の updater 設定:

```json
{
  "plugins": {
    "updater": {
      "pubkey": "<~/.tauri/promptnotes.key.pub の中身（改行含む）>"
    }
  }
}
```

```bash
# 3. 秘密鍵を nix-sops で暗号化して git 管理（既存の age key を使う）
mkdir -p secrets/tauri
sops --encrypt --input-type binary --output-type yaml \
  ~/.tauri/promptnotes.key > secrets/tauri/updater.key.sops.yaml

# 4. 平文の秘密鍵は削除（nix-sops から復元可能）
shred -u ~/.tauri/promptnotes.key
rm ~/.tauri/promptnotes.key.pub  # 公開鍵は tauri.conf.json に埋め込み済み

# 5. secrets/tauri/updater.key.sops.yaml を git commit
git add secrets/tauri/updater.key.sops.yaml
```

#### macOS へ秘密鍵を配置

NixOS の `secrets/tauri/updater.key.sops.yaml` を macOS の Nix flake 経由で復号し、`~/.tauri/promptnotes.key` に配置する。Nix + nix-sops の運用は既存のものを使う（手順は nix-sops の慣習に従う）。

#### build 時に使う

Tauri の updater 署名は環境変数で渡す。`TAURI_SIGNING_PRIVATE_KEY` には鍵ファイルのパスまたは鍵の内容を直接指定できる。`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` にはパスフレーズを直接指定する。

環境変数は **ターミナルを閉じる / OS を再起動すると消える** ため、ビルドするたびに同じターミナル内で再設定が必要。

##### NixOS (メイン機)

パスフレーズは `pass` (GPG-based password manager) で管理する。

```bash
# updater 署名用の秘密鍵を復号
mkdir -p ~/.tauri
sops --decrypt secrets/tauri/updater.key.sops.yaml | sed 's/^data: //' > ~/.tauri/promptnotes.key
chmod 600 ~/.tauri/promptnotes.key

# 環境変数を設定 (パスフレーズは pass から取得)
export TAURI_SIGNING_PRIVATE_KEY="$HOME/.tauri/promptnotes.key"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="$(pass <pass entry name>)"

bun run tauri build --bundles deb,appimage,rpm
```

##### macOS (サブ機)

パスフレーズは [Bitwarden CLI](https://bitwarden.com/help/cli/) (`bw`) で管理する。Secure Note として「Tauri Auto Updater Pass phrase」という名前で保存済みの前提。

```bash
# 1. Bitwarden CLI をインストール (初回のみ)
brew install bw

# 2. ログイン (初回のみ). 以降はセッションが有効な間再利用可能
bw login

# 3. セッションを取得 (ターミナルを開くたびに必要)
export BW_SESSION="$(bw unlock --raw)"

# 4. updater 署名用の秘密鍵を sops で復号
mkdir -p ~/.tauri
sops --decrypt secrets/tauri/updater.key.sops.yaml | sed 's/^data: //' > ~/.tauri/promptnotes.key
chmod 600 ~/.tauri/promptnotes.key

# 5. Bitwarden からパスフレーズを取得
ITEM_ID=$(bw list items --search "Tauri Auto Updater Pass phrase" --session "$BW_SESSION" | jq -r '.[0].id')
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="$(bw get item "$ITEM_ID" --session "$BW_SESSION" | jq -r '.notes')"

# 6. 秘密鍵のパスを指定
export TAURI_SIGNING_PRIVATE_KEY="$HOME/.tauri/promptnotes.key"

# 確認 (任意)
echo "key: $TAURI_SIGNING_PRIVATE_KEY"
echo "pass: ${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:0:4}..."

# 7. ビルド実行
cd apps/promptnotes
nix develop
bun install
bun run tauri build --bundles dmg
```

> **環境変数のスコープ**: `export` は現在のシェルセッションのみ有効。`nix develop` の内側と外側で環境変数が分離される場合があるため、`nix develop` に入ってから `export` する方が確実。ターミナルを閉じると消えるので、ビルド終了後はターミナルを閉じることで誤った残存を防げる。

> **BW_SESSION**: `bw unlock --raw` で取得したセッションキーは一定時間 (デフォルト1時間) 有効。連続作業中は `bw unlock` を繰り返さず、同じ `BW_SESSION` を使い回せる。

> **パスワード管理**: updater key のパスフレーズは `pass` (NixOS、GPG-based) または Bitwarden (macOS) で安全に保管すること。パスフレーズを失った場合も秘密鍵を再生成すれば復旧可能 (公開鍵も差し替えが必要)。秘密鍵自体を失った場合はアップデート発行が永久に不可能になるため、nix-sops による git 管理が安全網になる。

### 3.4 やってはいけないこと

- **`openssl genpkey` で鍵生成しない**: Tauri の updater は minisign 形式を期待する。必ず `cargo tauri signer generate` を使うこと
- **Developer ID を取得しない** (Apple Developer Program に参加しないため)。コード署名も行わない
- **updater keypair を platform ごとに生成しない**: NixOS と macOS で署名鍵が変わると検証が壊れる
- **秘密鍵を nix-sops に暗号化せずに平文で git commit しない**
- **秘密鍵のバックアップを怠らない**: nix-sops で暗号化して git 管理することでバックアップを兼ねるが、念のため 1Password 等のパスワードマネージャーにも平文を保管しておくこと。秘密鍵を失うと以後のアップデート発行が永久に不可能になる

---

## 4. ローカルビルド手順

### 4.1 Linux build (NixOS メイン機)

> Linux build entrypoint: [.github/workflows/build-appimage.yml](../.github/workflows/build-appimage.yml)

```bash
# updater 署名用の秘密鍵を復号
mkdir -p ~/.tauri
sops --decrypt secrets/tauri/updater.key.sops.yaml | sed 's/^data: //' > ~/.tauri/promptnotes.key
chmod 600 ~/.tauri/promptnotes.key

# 環境変数を設定 (パスフレーズは pass から取得)
export TAURI_SIGNING_PRIVATE_KEY="$HOME/.tauri/promptnotes.key"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="$(pass <pass entry name>)"

# dev shell に入ってからビルド
cd apps/promptnotes
nix develop
bun install
bun run tauri build --bundles deb,appimage,rpm
```

成果物: `apps/promptnotes/src-tauri/target/release/bundle/{deb,rpm,appimage}/`

#### 4.1.1 Nix flake 配布ビルド

Nix flake は、**Nix エコシステム内での配布形式**として機能する。
`nix build` の成果物は `/nix/store/` に依存したラップ済みバイナリであり、一般 Linux 環境へのポータブル配布には使えない（AppImage/.deb/.rpm が必要）。

Nix ユーザは flake を直接参照することで、ソースからのビルド・実行が可能:

```bash
# flake を直接実行 (fetch → build → run)
nix run github:dev-komenzar/promptnotes

# プロファイルにインストール
nix profile install github:dev-komenzar/promptnotes
```

`wrapProgram` により `LD_LIBRARY_PATH` / `GSETTINGS_SCHEMA_DIR` / `XDG_DATA_DIRS` が
Nix store のパスに解決され、NixOS / nix-darwin 環境で動作する。

将来的には nixpkgs への提出や [crane-tauri](https://github.com/JPHutchins/crane-tauri) 導入による差分ビルド効率化を検討する。

### 4.2 macOS build (mac サブ機)

> 環境変数の取得手順の詳細は [3.3 build 時に使う](#build-時に使う) の「macOS (サブ機)」節を参照。以下は要点のみ。

```bash
cd apps/promptnotes

# Bitwarden からパスフレーズを取得 (ターミナルを開くたびに必要)
export BW_SESSION="$(bw unlock --raw)"
ITEM_ID=$(bw list items --search "Tauri Auto Updater Pass phrase" --session "$BW_SESSION" | jq -r '.[0].id')
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="$(bw get item "$ITEM_ID" --session "$BW_SESSION" | jq -r '.notes')"

# updater 署名用の秘密鍵を sops で復号
mkdir -p ~/.tauri
sops --decrypt secrets/tauri/updater.key.sops.yaml | sed 's/^data: //' > ~/.tauri/promptnotes.key
chmod 600 ~/.tauri/promptnotes.key

# 秘密鍵のパスを指定
export TAURI_SIGNING_PRIVATE_KEY="$HOME/.tauri/promptnotes.key"

# nix develop の内側に入ってからビルド
nix develop
bun install
bun run tauri build --bundles dmg
# OR if you want explicit app bundle without dmg:
# bun run tauri build --bundles app
```

`createUpdaterArtifacts: true` により、Tauri は `.dmg` に加えて updater 用アーティファクトも自動生成する。

成果物:

```
apps/promptnotes/src-tauri/target/release/bundle/macos/*.dmg
apps/promptnotes/src-tauri/target/release/bundle/macos/*.app.tar.gz
apps/promptnotes/src-tauri/target/release/bundle/macos/*.app.tar.gz.sig
apps/promptnotes/src-tauri/target/release/bundle/macos/latest.json  (macOS 用; Linux CI のと統合必須)
```

### 4.3 release の一連の流れ

1. version bump (`apps/promptnotes/package.json` と `apps/promptnotes/src-tauri/Cargo.toml`)
2. `git tag vX.Y.Z` & push
3. Linux CI をトリガー: tag を push すると `.github/workflows/build-appimage.yml` が起動し、AppImage / .deb / .rpm / .AppImage.sig / latest.json (Linux 分) を draft Release に自動アップロード
4. macOS ローカルビルド: mac サブ機で `bun run tauri build --bundles dmg` を実行し、.dmg / .app.tar.gz / .app.tar.gz.sig / latest.json (macOS 分) を同一の draft Release にアップロード
5. latest.json を統合: [4.4](#44-latestjson-merge-protocol) の手順に従い、Linux CI と macOS の `latest.json` をマージして --clobber でアップロード
6. release notes に **macOS 初回起動時の Gatekeeper 回避手順** を必ず記載
7. publish

> **Nix flake について**: flake は GitHub リポジトリ自体が配布経路のため、成果物のアップロードは不要。
> `git tag` を打てばユーザは `nix run github:dev-komenzar/promptnotes/<tag>` で特定バージョンを参照できる。

Tauri の updater は GitHub Releases を配信元にする想定。
updater の署名検証は Developer ID / notarization と独立なので、Apple Developer Program 不参加でも Tauri updater は問題なく機能する。

Linux の updater はバンドル形式 (AppImage / .deb / .rpm) を自動検出し、`latest.json` の対応キー (`linux-x86_64` / `linux-x86_64-deb` / `linux-x86_64-rpm`) から適切なアセットをダウンロードする。
AppImage は root 不要でファイル置換。.deb/.rpm は `dpkg -i` / `rpm -U` を実行するためユーザに sudo パスワードを要求する。

#### GitHub Releases へのアップロード手順

##### Linux 成果物のアップロード

```bash
VERSION="0.2.0"  # package.json / Cargo.toml と一致させる

# draft release を作成 (まだ公開しない)
gh release create "v${VERSION}" \
  --title "PromptNotes v${VERSION}" \
  --notes "リリースノートをここに記述" \
  --draft

# AppImage
gh release upload "v${VERSION}" \
  apps/promptnotes/src-tauri/target/release/bundle/appimage/*.AppImage \
  apps/promptnotes/src-tauri/target/release/bundle/appimage/*.AppImage.sig

# .deb
gh release upload "v${VERSION}" \
  apps/promptnotes/src-tauri/target/release/bundle/deb/*.deb

# .rpm
gh release upload "v${VERSION}" \
  apps/promptnotes/src-tauri/target/release/bundle/rpm/*.rpm
```

> `.sig` ファイルは updater の署名検証に必須。`.deb` / `.rpm` はパッケージマネージャ経由の更新に使われる（個別の `.sig` は不要）。

##### macOS 成果物のアップロード

```bash
VERSION="0.2.0"

# .dmg をアップロード
gh release upload "v${VERSION}" \
  apps/promptnotes/src-tauri/target/release/bundle/dmg/*.dmg
```

> **`.app.tar.gz` と `.sig` について**:
>
> macOS 向けには 2 種類の成果物が必要:
>
> | 成果物 | 用途 | 生成条件 |
> |---|---|---|
> | `.dmg` | ユーザが GitHub Releases から手動ダウンロードしてインストール | 常に生成 |
> | `.app.tar.gz` + `.sig` | Tauri in-app updater が自動更新に使う | `createUpdaterArtifacts: true` 時のみ |
>
> `.app.tar.gz` は `.app` バンドルを tar+gzip でアーカイブしたもの。Tauri の updater プラグインは起動時に `latest.json` を取得し、新バージョンがあれば `.app.tar.gz` をダウンロード → `.sig` で署名検証 → アプリを差し替える。`.sig` は minisign 形式の電子署名で、ダウンロードしたバイナリが改ざんされていないことを保証する。
>
> `.dmg` だけアップロードしても in-app updater は動作しない。必ず `.app.tar.gz` と `.sig` もアップロードすること。生成先は `apps/promptnotes/src-tauri/target/release/bundle/macos/`。
>
> ```bash
> gh release upload "v${VERSION}" \
>   apps/promptnotes/src-tauri/target/release/bundle/macos/*.app.tar.gz \
>   apps/promptnotes/src-tauri/target/release/bundle/macos/*.app.tar.gz.sig
> ```

##### latest.json のアップロードと統合

`createUpdaterArtifacts: true` を設定している場合、Tauri build が `latest.json` を生成する。これをリリースにアップロードすることで in-app updater が更新を検出できる。

Linux CI と macOS ローカルビルドは**それぞれ独立して `latest.json` を生成**し、自動統合されない。両方のビルドが完了した後、[4.4](#44-latestjson-merge-protocol) の手順に従ってマージしてからアップロードする。

```bash
# latest.json の生成場所を確認 (macOS)
ls apps/promptnotes/src-tauri/target/release/bundle/macos/latest.json
```

> **重要**: 後からビルドした方の `latest.json` をそのままアップロードすると、先にアップロードした platform の署名が失われる。必ずマージしてからアップロードすること。

##### リリースの公開

全 platform の成果物と `latest.json` が揃ったら、draft を公開する:

```bash
gh release edit "v${VERSION}" --draft=false
```

##### リリースノートのテンプレート

```markdown
## 変更内容

- 変更点を箇条書きで記述

## インストール

各 platform 向けのインストール手順は [README](https://github.com/dev-komenzar/promptnotes#インストール) を参照。

### macOS での起動

promptnotes は Apple Developer Program に登録していないため、公証 (notarization) されていません。
初回起動時に Gatekeeper の警告が出ます。以下のいずれかで回避してください:

**方法 A: 右クリックで開く**
1. Finder で `promptnotes.app` を右クリック (control + クリック)
2. 「開く」を選択
3. 再度警告が出るが、もう一度「開く」をクリック

**方法 B: ターミナルから属性を解除**
`xattr -dr com.apple.quarantine /Applications/promptnotes.app`
```

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

### 4.4 latest.json merge protocol

Linux CI (`.github/workflows/build-appimage.yml`) と macOS ローカルビルドは**それぞれ独立して動作し、各 platform 用の `latest.json` を別々に生成する**。Tauri の updater ツールチェーンは platform 間で `latest.json` を自動統合しない。そのため、両方のビルドが完了した後、人手で 2 つの `latest.json` をマージする必要がある。

**上書きの危険**: 後からビルドした方の `latest.json` をそのままアップロードすると、先にアップロードした platform の署名が失われる。in-app updater がその platform で動作しなくなる。

#### マージ手順

1. Linux CI の draft Release から `latest.json` をダウンロード:
   ```bash
   VERSION="0.2.0"
   gh release download "v${VERSION}" --pattern latest.json --dir /tmp/merge-latest
   cp /tmp/merge-latest/latest.json /tmp/merge-latest/linux.json
   ```

2. Linux のキーのみ抽出:
   ```bash
   jq '{platforms: { "linux-x86_64": .platforms."linux-x86_64", "linux-x86_64-deb": .platforms."linux-x86_64-deb", "linux-x86_64-rpm": .platforms."linux-x86_64-rpm" }}' \
     /tmp/merge-latest/linux.json > /tmp/merge-latest/linux-only.json
   ```

3. macOS ローカルビルドで生成された `latest.json` を確認:
   ```bash
   ls apps/promptnotes/src-tauri/target/release/bundle/macos/latest.json
   # Darwin のキーのみ抽出
   jq '{platforms: { "darwin-aarch64": .platforms."darwin-aarch64" }}' \
     apps/promptnotes/src-tauri/target/release/bundle/macos/latest.json \
     > /tmp/merge-latest/mac-only.json
   ```

4. 両方を統合:
   ```bash
   jq -s '.[0].platforms * .[1].platforms | {version: ("v'"${VERSION}"'"), notes: "", platforms: .}' \
     /tmp/merge-latest/linux-only.json \
     /tmp/merge-latest/mac-only.json \
     > /tmp/merge-latest/merged-latest.json
   ```

   > **スキーマの確認**: Tauri が生成する実際の `latest.json` の構造は `cat apps/promptnotes/src-tauri/target/release/bundle/macos/latest.json` で確認できる。スキーマが異なる場合は上記 `jq` コマンドを実際の構造に合わせて調整すること。

5. 統合した `latest.json` を draft Release に上書きアップロード:
   ```bash
   gh release upload "v${VERSION}" --clobber /tmp/merge-latest/merged-latest.json#latest.json
   ```

このマージ手順は **draft Release を公開する前に必ず実行すること**。公開後に `latest.json` を差し替えても、既にチェックしたクライアントは古い `latest.json` をキャッシュしている可能性がある。

---

## 5. 補足

### 5.1 Windows を MVP から除外する理由

- 物理マシンを保有していない
- Wine + `osslsigncode` でクロス sign は可能だが、本物の EV cert フロー (SmartScreen 即時信頼等) は結局 Windows 環境が必要
- 個人開発のリソース制約上、優先度は mac / Linux に置く

### 5.2 CI 移行の判断基準

現状: Linux のリリースビルドは GitHub Actions (ubuntu-22.04) をプライマリパスとして使用している。macOS はローカルビルドを継続。

以下のいずれかに該当したら macOS も CI へ移行する:

- mac サブ機の電源を入れる頻度が月に数回以下になり、release のたびに起動が面倒
- 共同開発者が増え、複数マシンで同時 release 検証が必要になった
- Windows 対応を再開する

その場合は:

- macOS build は GitHub Actions の **有料 mac runner** ($0.08/min) をスポット利用
- private secret は GitHub Actions Secrets に移し、nix-sops repo は個人用として分離維持

### 5.3 配布戦略

#### Linux 配布形式 (決定: `ori-wuxk`)

| 形式 | 配布経路 | 構築コスト | updater 対応 | 備考 |
|---|---|---|---|---|
| **AppImage** | GitHub Releases | ゼロ (Tauri bundler ビルトイン) | ✅ ファイル置換・root 不要 | 全ディストロ対応。ポータブル用途に最適。`.AppImage.sig` は CI で自動生成・アップロード |
| **.deb** | GitHub Releases | ゼロ | ✅ `dpkg -i`・root 必須 | Debian/Ubuntu/Pop!_OS 向け |
| **.rpm** | GitHub Releases | ゼロ | ✅ `rpm -U`・root 必須 | Fedora/RHEL/openSUSE 向け |
| **Nix flake** | flake.nix 経由 | 既存 flake を流用 | ❌ (flake update で更新) | NixOS / nix-darwin 向け。`nix profile install` |

**採用しなかった形式:**

| 形式 | 不採用理由 |
|---|---|
| **Flatpak** | flatpak-builder マニフェスト手動作成・オフラインビルド対応に 2〜5 人日の初期コスト。コミュニティ要望があれば再検討 |
| **Snap** | 起動が遅く、Canonical 管理ストア、Tauri コミュニティでの採用率が低い。コミュニティ要望があれば再検討 |
| **AUR** | Arch Linux 向け。コミュニティ主導の PKGBUILD で十分。自前管理しない |

> **ビルド環境に関する重要な制約**: Tauri 公式ドキュメントが推奨するように、AppImage/.deb/.rpm は**サポートする最古のシステム**でビルドする必要がある。新しすぎる glibc でビルドすると古いディストロで起動できなくなる。現状は NixOS (最新) でビルドしているため、広範囲のディストロ対応には Ubuntu 22.04 ベースの CI 環境が将来的に必要になる可能性がある。

#### macOS 配布 (notarization なし)

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