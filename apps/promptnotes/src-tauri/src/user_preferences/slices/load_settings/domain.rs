use std::path::PathBuf;

/// `load-settings` slice の input。`config_path` は composition root が解決した
/// `app_config_dir/settings.json` の絶対パス。
#[derive(Debug, Clone)]
pub struct LoadSettingsCommand {
    pub config_path: PathBuf,
}
