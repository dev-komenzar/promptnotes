use serde::{Deserialize, Serialize};

/// NoteFeed の sort 基準。Settings に永続化される。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortField {
    #[default]
    CreatedAt,
    UpdatedAt,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortDirection {
    Asc,
    #[default]
    Desc,
}

/// `SortField` × `SortDirection`。I-S3 デフォルトは `{ CreatedAt, Desc }`。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SortOrder {
    field: SortField,
    direction: SortDirection,
}

impl SortOrder {
    pub const fn new(field: SortField, direction: SortDirection) -> Self {
        Self { field, direction }
    }

    pub const fn field(&self) -> SortField {
        self.field
    }

    pub const fn direction(&self) -> SortDirection {
        self.direction
    }
}
