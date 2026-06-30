{
  description = "PromptNotes — Tauri v2 + SvelteKit + Bun dev shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

        # Tauri v2 Linux runtime/build deps (per https://v2.tauri.app/start/prerequisites/)
        linuxDeps = with pkgs; [
          webkitgtk_4_1
          gtk3
          libsoup_3
          openssl
          librsvg
          libayatana-appindicator
          xdotool
          glib
          gsettings-desktop-schemas
        ];

        # vitest browser (Playwright chromium headless shell) runtime libs.
        # Playwright downloads its own chromium to ~/.cache/ms-playwright but the
        # binary dlopens system libs (libglib, libnss, libdrm, libxkbcommon, ...)
        # that NixOS does not put on the default loader path. We expose them via
        # LD_LIBRARY_PATH without coupling to a specific chromium revision.
        chromiumDeps = with pkgs; [
          glib nss nspr at-spi2-atk at-spi2-core atk
          cups dbus libdrm expat libxkbcommon
          mesa libgbm
          pango cairo alsa-lib
          libxcb libx11 libxcomposite libxdamage
          libxext libxfixes libxrandr
        ];

        darwinDeps = with pkgs; [
          libiconv
        ];

        commonTools = with pkgs; [
          rustToolchain
          bun
          nodejs_22
          cargo-tauri
          pkg-config
        ];

        # apm CLI は PyInstaller でバンドルされており、ctypes / sqlite3 などの
        # Python 標準モジュールが libffi / libsqlite3 / zlib 等の共有ライブラリを
        # 実行時 dlopen する。NixOS では LD_LIBRARY_PATH で明示する必要がある。
        apmRuntimeLibs = with pkgs; [
          libffi
          sqlite
          zlib
          bzip2
          xz
          openssl
          stdenv.cc.cc.lib
        ];
        gsettingsSchemaDir = "${pkgs.gsettings-desktop-schemas}/share/gsettings-schemas/${pkgs.gsettings-desktop-schemas.name}/glib-2.0/schemas:${pkgs.gtk3}/share/gsettings-schemas/${pkgs.gtk3.name}/glib-2.0/schemas";

        # freedesktop .desktop file (heredoc indent 罠回避のため makeDesktopItem を使用)
        promptnotesDesktopItem = pkgs.makeDesktopItem {
          name = "promptnotes";
          desktopName = "PromptNotes";
          comment = "AI prompt notes desktop app";
          exec = "promptnotes";
          icon = "promptnotes";
          terminal = false;
          type = "Application";
          categories = [ "Utility" "Office" ];
        };

        # FOD: bun dependencies (node_modules tarball)
        # bun install は sandbox 内でネットワーク不可なため、FOD で先に deps を
        # fetch する。出力 hash で検証されるため FOD は network access を許可される。
        # node_modules は shebang に nix store path を含むため、directory 出力だと
        # FOD の参照制約に引っかかる。tarball で出力して回避する。
        bunDeps = pkgs.stdenvNoCC.mkDerivation {
          name = "promptnotes-bun-deps";

          # package.json と bun.lock のみ入力 (source code に依存しない)
          src = pkgs.runCommand "promptnotes-bun-inputs" {} ''
            mkdir $out
            cp ${./apps/promptnotes/package.json} $out/package.json
            cp ${./apps/promptnotes/bun.lock} $out/bun.lock
          '';

          impureEnvVars = pkgs.lib.fetchers.proxyImpureEnvVars;

          nativeBuildInputs = with pkgs; [
            bun
            nodejs_22
            cacert
          ];

          buildPhase = ''
            runHook preBuild
            export HOME=$(mktemp -d)
            export BUN_INSTALL_CACHE_DIR=$HOME/.bun/cache
            # --ignore-scripts: prepare (svelte-kit sync) は本 build で実行するため FOD では skip
            bun install --frozen-lockfile --ignore-scripts
            runHook postBuild
          '';

          installPhase = ''
            runHook preInstall
            tar --sort=name --mtime='@0' --owner=0 --group=0 --numeric-owner \
              -cf $out node_modules
            runHook postInstall
          '';

          outputHashMode = "flat";
          outputHash = "sha256-qpSxAQecwMpaMuEVXWhMGhhQ5oB3MpVF5LduCu6N3nY=";
        };

        # FOD: cargo dependencies (vendored crates)
        # fetchCargoVendor が Cargo.lock から crate を download して vendor dir を生成する。
        cargoDeps = pkgs.rustPlatform.fetchCargoVendor {
          name = "promptnotes-cargo-deps";
          src = ./apps/promptnotes/src-tauri;
          hash = "sha256-SkgBZNSqgi3tltwXsS8uEOc+NzS1p2OUkQAuWXIO8r0=";
        };

        # NixOS 個人利用向けの PromptNotes パッケージ。
        # bun install / cargo fetch は上記 FOD で事前実行するため、本 derivation は
        # pure sandbox で network access 不要 (sandbox = relaxed も不要)。
        promptnotesPackage = pkgs.stdenv.mkDerivation {
          pname = "promptnotes";
          version = "0.2.0";
          src = ./.;

          nativeBuildInputs = with pkgs; [
            rustToolchain
            cargo-tauri
            bun
            nodejs_22
            pkg-config
            makeWrapper
          ] ++ pkgs.lib.optionals pkgs.stdenv.isLinux [
            pkgs.wrapGAppsHook3
          ];

          buildInputs = pkgs.lib.optionals pkgs.stdenv.isLinux linuxDeps
            ++ pkgs.lib.optionals pkgs.stdenv.isDarwin darwinDeps;

          dontWrapGApps = true;

          configurePhase = ''
            runHook preConfigure
            export HOME=$(mktemp -d)
            export CARGO_HOME=$HOME/.cargo
            export XDG_CACHE_HOME=$HOME/.cache
            export CARGO_NET_OFFLINE=true

            cd apps/promptnotes

            # FOD で事前 fetch した node_modules tarball を展開 (network 不要)
            tar -xf ${bunDeps} -C .
            chmod -R +w ./node_modules

            # sandbox 内に /usr/bin/env が無いため、node_modules 内の
            # shebang を nix store path に patch する。
            # .bin/ の symlink target にも実行権限が必要なため全体に +x 付与。
            chmod -R +x ./node_modules
            patchShebangs ./node_modules

            # prepare script (svelte-kit sync) は FOD で skip したためここで実行
            bun run prepare || true

            # cargo が FOD の vendored crates を使うよう .cargo/config.toml を設定
            # fetchCargoVendor は source-registry-0/ サブディレクトリに crates を配置する
            mkdir -p src-tauri/.cargo
            cat > src-tauri/.cargo/config.toml <<EOF
            [source.crates-io]
            replace-with = "vendored-sources"

            [source.vendored-sources]
            directory = "${cargoDeps}/source-registry-0"
            EOF

            runHook postConfigure
          '';

          buildPhase = ''
            runHook preBuild
            # cargo tauri build は tauri.conf.json の beforeBuildCommand
            # (bun run build) を自動で呼ぶ。--no-bundle で deb/AppImage 等の
            # bundling は省き、生 binary のみ生成する (Nix 側で wrap するため)。
            # CARGO_NET_OFFLINE=true により cargo は network に access しない。
            cargo tauri build --no-bundle
            runHook postBuild
          '';

          installPhase = ''
            runHook preInstall
            mkdir -p $out/bin
            install -Dm755 src-tauri/target/release/app $out/bin/promptnotes
          '' + pkgs.lib.optionalString pkgs.stdenv.isLinux ''
            # XDG hicolor icon theme: 実 PNG サイズと path サイズを一致させる
            # (cargo tauri icon が生成した 32/64/128/256/512 を全て展開)
            install -Dm644 src-tauri/icons/32x32.png \
              $out/share/icons/hicolor/32x32/apps/promptnotes.png
            install -Dm644 src-tauri/icons/64x64.png \
              $out/share/icons/hicolor/64x64/apps/promptnotes.png
            install -Dm644 src-tauri/icons/128x128.png \
              $out/share/icons/hicolor/128x128/apps/promptnotes.png
            install -Dm644 src-tauri/icons/128x128@2x.png \
              $out/share/icons/hicolor/256x256/apps/promptnotes.png
            install -Dm644 src-tauri/icons/icon.png \
              $out/share/icons/hicolor/512x512/apps/promptnotes.png
            install -Dm644 ${promptnotesDesktopItem}/share/applications/promptnotes.desktop \
              $out/share/applications/promptnotes.desktop
          '' + ''
            runHook postInstall
          '';

          postFixup = pkgs.lib.optionalString pkgs.stdenv.isLinux ''
            wrapProgram $out/bin/promptnotes \
              --set GSETTINGS_SCHEMA_DIR "${gsettingsSchemaDir}" \
              --prefix XDG_DATA_DIRS : "${pkgs.gsettings-desktop-schemas}/share:${pkgs.gtk3}/share" \
              --prefix LD_LIBRARY_PATH : "${pkgs.lib.makeLibraryPath linuxDeps}" \
              "''${gappsWrapperArgs[@]}"
          '';

          meta = with pkgs.lib; {
            description = "PromptNotes — AI プロンプトを書き溜めてすぐコピーできるローカル Tauri アプリ";
            homepage = "https://github.com/dev-komenzar/promptnotes";
            mainProgram = "promptnotes";
            platforms = platforms.linux ++ platforms.darwin;
          };
        };
      in
      {
        devShells.default = pkgs.mkShell {
          packages = commonTools
            ++ pkgs.lib.optionals pkgs.stdenv.isLinux linuxDeps
            ++ pkgs.lib.optionals pkgs.stdenv.isDarwin darwinDeps;

          shellHook = pkgs.lib.optionalString pkgs.stdenv.isLinux ''
            # FileChooser ($_GLib-GIO-WARNING: ... gtk schemas) workaround on NixOS
            export XDG_DATA_DIRS="${pkgs.gsettings-desktop-schemas}/share:${pkgs.gtk3}/share:$XDG_DATA_DIRS"
            export GSETTINGS_SCHEMA_DIR="${gsettingsSchemaDir}"
            # apm CLI (PyInstaller) + Playwright chromium headless shell need
            # libffi / libsqlite3 / libglib / libnss / libdrm / libxkbcommon / ...
            export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath (apmRuntimeLibs ++ chromiumDeps)}:$LD_LIBRARY_PATH"
            # Blank-window mitigation on some GPU/driver combos (uncomment if you see it)
            # export WEBKIT_DISABLE_COMPOSITING_MODE=1
          '' + ''
            echo "PromptNotes dev shell: Rust $(rustc --version | cut -d' ' -f2) / Bun $(bun --version) / Node $(node --version)"
          '';
        };

        packages.default = promptnotesPackage;
        packages.promptnotes = promptnotesPackage;

        apps.default = {
          type = "app";
          program = "${promptnotesPackage}/bin/promptnotes";
        };
      });
}
