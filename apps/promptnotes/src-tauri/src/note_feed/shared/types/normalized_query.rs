use unicode_normalization::UnicodeNormalization;

/// Note Feed BC の正規化済み検索文字列 (`aggregates.md#note-feed-aggregate-elements`)。
///
/// I-F1 / C-UF1 を smart constructor で施行する: 入力を **NFKC** 正規化 + lowercase 化し、
/// `trim` 後 empty なら `None` に降格する (C-UF2)。**生の入力文字列は保持しない** (C-UF8)。
///
/// Note: domain 文書は "NFC" と記載するが、S8 シナリオの walkthrough (全角 `Ｇｐｔ` → 半角 `gpt`)
/// が成立するには **NFKC** (compatibility normalization) が必要。本 slice は walkthrough の
/// 意図を採用し NFKC を使う。terminology のずれは follow-up issue で domain 提案する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedQuery(String);

impl NormalizedQuery {
    /// 入力を NFKC 正規化 + lowercase 化。空白のみ / 空文字は `None` に降格 (C-UF2)。
    pub fn from_raw(raw: &str) -> Option<Self> {
        let normalized: String = raw.nfkc().collect::<String>().to_lowercase();
        let trimmed = normalized.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(Self(trimmed.to_string()))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
