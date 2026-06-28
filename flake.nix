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
      in
      {
        devShells.default = pkgs.mkShell {
          packages = commonTools
            ++ pkgs.lib.optionals pkgs.stdenv.isLinux linuxDeps
            ++ pkgs.lib.optionals pkgs.stdenv.isDarwin darwinDeps;

          shellHook = pkgs.lib.optionalString pkgs.stdenv.isLinux ''
            # FileChooser ($_GLib-GIO-WARNING: ... gtk schemas) workaround on NixOS
            export XDG_DATA_DIRS="${pkgs.gsettings-desktop-schemas}/share:${pkgs.gtk3}/share:$XDG_DATA_DIRS"
            export GSETTINGS_SCHEMA_DIR="${pkgs.gsettings-desktop-schemas}/share/gsettings-schemas/${pkgs.gsettings-desktop-schemas.name}/glib-2.0/schemas:${pkgs.gtk3}/share/gsettings-schemas/${pkgs.gtk3.name}/glib-2.0/schemas"
            # apm CLI (PyInstaller) + Playwright chromium headless shell need
            # libffi / libsqlite3 / libglib / libnss / libdrm / libxkbcommon / ...
            export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath (apmRuntimeLibs ++ chromiumDeps)}:$LD_LIBRARY_PATH"
            # Blank-window mitigation on some GPU/driver combos (uncomment if you see it)
            # export WEBKIT_DISABLE_COMPOSITING_MODE=1
          '' + ''
            echo "PromptNotes dev shell: Rust $(rustc --version | cut -d' ' -f2) / Bun $(bun --version) / Node $(node --version)"
          '';
        };
      });
}
