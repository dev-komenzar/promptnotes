use serde::{Deserialize, Serialize};

/// UI テーマ (Q4 で確定の System / Light / Dark)。I-S3 デフォルトは `System`。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}
