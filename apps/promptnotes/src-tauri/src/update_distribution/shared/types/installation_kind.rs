/// updater が install できる配布形式 (`aggregates.md#update-channel-aggregate-elements`)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BundleFormat {
    MacApp,
    AppImage,
    Deb,
    Rpm,
    Msi,
    Nsis,
}

/// 実行中アプリの配布形式 (I-U6)。bundle 時に埋め込まれ immutable。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallationKind {
    Bundled(BundleFormat),
    /// bundle 種別を持たないビルド (nix package / `--no-bundle` / 開発ビルド)
    Unbundled,
}

impl InstallationKind {
    /// I-U6: in-app update を install できるか。通知 UI のボタン表示判定にも使う。
    pub fn is_installable(&self) -> bool {
        matches!(self, Self::Bundled(_))
    }
}
