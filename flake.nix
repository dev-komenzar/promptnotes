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
            # Blank-window mitigation on some GPU/driver combos (uncomment if you see it)
            # export WEBKIT_DISABLE_COMPOSITING_MODE=1
          '' + ''
            echo "PromptNotes dev shell: Rust $(rustc --version | cut -d' ' -f2) / Bun $(bun --version) / Node $(node --version)"
          '';
        };
      });
}
