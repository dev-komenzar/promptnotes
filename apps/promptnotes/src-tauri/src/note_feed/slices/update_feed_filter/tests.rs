//! Tests for slice `update-feed-filter`.
//!
//! Spec: `.ori/slices/update-feed-filter/spec.md#test-perspectives`.
//!
//! 設計メモ:
//! - 本 slice は副作用ゼロ (C-UF6): Repository / EventBus を inject しない。
//!   `UpdateFeedFilterUseCase::apply` のシグネチャが `(NoteFeed, cmd) -> NoteFeed` 1 本である事自体が
//!   read model / 揮発 / no domain event を type-level に表現する (TP-SE1)。
//! - GREEN 化は phase 4 (impl-green) の責務。RED 段階では未実装 module の path 解決失敗を期待する。

use proptest::prelude::*;

use crate::note_capture::shared::types::Tag;
use crate::note_feed::shared::types::{DateRangeFilter, FeedFilter, NoteFeed};

use super::application::UpdateFeedFilterUseCase;
use super::domain::UpdateFeedFilterCommand;

fn empty_feed() -> NoteFeed {
    NoteFeed::empty()
}

fn tag(raw: &str) -> Tag {
    Tag::new(raw).expect("test tag must be valid")
}

// ===== TP-Q*: SetQuery =====

/// spec.md#tp-set-query TP-Q1 — "GPT" → Some(NormalizedQuery("gpt"))
#[test]
fn tp_q1_uppercase_query_is_lowercased() {
    let feed = empty_feed();
    let out = UpdateFeedFilterUseCase::apply(
        feed,
        UpdateFeedFilterCommand::SetQuery {
            raw: "GPT".to_string(),
        },
    );
    let q = out.filter().query().expect("query must be Some");
    assert_eq!(q.as_str(), "gpt");
}

/// spec.md#tp-set-query TP-Q2 — "Ｇｐｔ" (全角) → "gpt" (NFC + lowercase)
#[test]
fn tp_q2_fullwidth_query_is_nfc_lowercased() {
    let feed = empty_feed();
    let out = UpdateFeedFilterUseCase::apply(
        feed,
        UpdateFeedFilterCommand::SetQuery {
            raw: "Ｇｐｔ".to_string(),
        },
    );
    let q = out.filter().query().expect("query must be Some");
    assert_eq!(
        q.as_str(),
        "gpt",
        "TP-Q2 / S8: 全角は NFC で半角化 + lowercase"
    );
}

/// spec.md#tp-set-query TP-Q3 — "" → None (空文字解除)
#[test]
fn tp_q3_empty_string_yields_none() {
    let feed = empty_feed();
    let out = UpdateFeedFilterUseCase::apply(
        feed,
        UpdateFeedFilterCommand::SetQuery { raw: String::new() },
    );
    assert!(out.filter().query().is_none());
}

/// spec.md#tp-set-query TP-Q4 — 空白のみ → None (C-UF2 trim 後 empty)
#[test]
fn tp_q4_whitespace_only_yields_none() {
    let feed = empty_feed();
    let out = UpdateFeedFilterUseCase::apply(
        feed,
        UpdateFeedFilterCommand::SetQuery {
            raw: "   \t\n".to_string(),
        },
    );
    assert!(out.filter().query().is_none());
}

/// spec.md#tp-set-query TP-Q5 — 連続適用で結果同一 (C-UF3 冪等)
#[test]
fn tp_q5_set_query_is_idempotent() {
    let first = UpdateFeedFilterUseCase::apply(
        empty_feed(),
        UpdateFeedFilterCommand::SetQuery {
            raw: "gpt".to_string(),
        },
    );
    let second = UpdateFeedFilterUseCase::apply(
        first.clone(),
        UpdateFeedFilterCommand::SetQuery {
            raw: "gpt".to_string(),
        },
    );
    assert_eq!(first, second);
}

/// spec.md#tp-set-query TP-Q6 — SetQuery は tag / date_range を保持 (C-UF5 直交性)
#[test]
fn tp_q6_set_query_preserves_other_axes() {
    // 事前に tag と date_range を入れた feed
    let base = UpdateFeedFilterUseCase::apply(
        empty_feed(),
        UpdateFeedFilterCommand::SetTag {
            tag: Some(tag("coding")),
        },
    );
    let base = UpdateFeedFilterUseCase::apply(
        base,
        UpdateFeedFilterCommand::SetDateRange {
            range: DateRangeFilter::Last7Days,
        },
    );

    let out = UpdateFeedFilterUseCase::apply(
        base,
        UpdateFeedFilterCommand::SetQuery {
            raw: "gpt".to_string(),
        },
    );

    assert_eq!(out.filter().tag(), Some(&tag("coding")));
    assert_eq!(out.filter().date_range(), &DateRangeFilter::Last7Days);
}

// ===== TP-D*: SetDateRange =====

/// spec.md#tp-set-date-range TP-D1 — Last7Days
#[test]
fn tp_d1_set_last_7_days() {
    let out = UpdateFeedFilterUseCase::apply(
        empty_feed(),
        UpdateFeedFilterCommand::SetDateRange {
            range: DateRangeFilter::Last7Days,
        },
    );
    assert_eq!(out.filter().date_range(), &DateRangeFilter::Last7Days);
}

/// spec.md#tp-set-date-range TP-D2 — All
#[test]
fn tp_d2_set_all() {
    let out = UpdateFeedFilterUseCase::apply(
        empty_feed(),
        UpdateFeedFilterCommand::SetDateRange {
            range: DateRangeFilter::All,
        },
    );
    assert_eq!(out.filter().date_range(), &DateRangeFilter::All);
}

/// spec.md#tp-set-date-range TP-D3 — Custom{from,to} 保持 (FeedDate VO 経由)
#[test]
fn tp_d3_set_custom_range_is_preserved() {
    let range =
        DateRangeFilter::custom_from_iso("2026-01-01", "2026-01-31").expect("valid custom range");
    let out = UpdateFeedFilterUseCase::apply(
        empty_feed(),
        UpdateFeedFilterCommand::SetDateRange {
            range: range.clone(),
        },
    );
    assert_eq!(out.filter().date_range(), &range);
}

/// spec.md#tp-set-date-range TP-D4 — SetDateRange は query / tag を保持 (C-UF5)
#[test]
fn tp_d4_set_date_range_preserves_other_axes() {
    let base = UpdateFeedFilterUseCase::apply(
        empty_feed(),
        UpdateFeedFilterCommand::SetQuery {
            raw: "gpt".to_string(),
        },
    );
    let base = UpdateFeedFilterUseCase::apply(
        base,
        UpdateFeedFilterCommand::SetTag {
            tag: Some(tag("rust")),
        },
    );

    let out = UpdateFeedFilterUseCase::apply(
        base,
        UpdateFeedFilterCommand::SetDateRange {
            range: DateRangeFilter::Last30Days,
        },
    );

    assert_eq!(
        out.filter().query().map(|q| q.as_str().to_string()),
        Some("gpt".to_string())
    );
    assert_eq!(out.filter().tag(), Some(&tag("rust")));
}

// ===== TP-T*: SetTag =====

/// spec.md#tp-set-tag TP-T1 — Some(Tag) セット
#[test]
fn tp_t1_set_some_tag() {
    let out = UpdateFeedFilterUseCase::apply(
        empty_feed(),
        UpdateFeedFilterCommand::SetTag {
            tag: Some(tag("coding")),
        },
    );
    assert_eq!(out.filter().tag(), Some(&tag("coding")));
}

/// spec.md#tp-set-tag TP-T2 — None で解除
#[test]
fn tp_t2_set_none_clears_tag() {
    let base = UpdateFeedFilterUseCase::apply(
        empty_feed(),
        UpdateFeedFilterCommand::SetTag {
            tag: Some(tag("coding")),
        },
    );
    let out = UpdateFeedFilterUseCase::apply(base, UpdateFeedFilterCommand::SetTag { tag: None });
    assert_eq!(out.filter().tag(), None);
}

/// spec.md#tp-set-tag TP-T3 — 同値 Tag 冪等 (C-UF7)
#[test]
fn tp_t3_set_same_tag_is_idempotent() {
    let first = UpdateFeedFilterUseCase::apply(
        empty_feed(),
        UpdateFeedFilterCommand::SetTag {
            tag: Some(tag("a")),
        },
    );
    let second = UpdateFeedFilterUseCase::apply(
        first.clone(),
        UpdateFeedFilterCommand::SetTag {
            tag: Some(tag("a")),
        },
    );
    assert_eq!(first, second);
}

/// spec.md#tp-set-tag TP-T4 — SetTag は query / date_range を保持 (C-UF5)
#[test]
fn tp_t4_set_tag_preserves_other_axes() {
    let base = UpdateFeedFilterUseCase::apply(
        empty_feed(),
        UpdateFeedFilterCommand::SetQuery {
            raw: "gpt".to_string(),
        },
    );
    let base = UpdateFeedFilterUseCase::apply(
        base,
        UpdateFeedFilterCommand::SetDateRange {
            range: DateRangeFilter::Last90Days,
        },
    );

    let out = UpdateFeedFilterUseCase::apply(
        base,
        UpdateFeedFilterCommand::SetTag {
            tag: Some(tag("rust")),
        },
    );

    assert_eq!(
        out.filter().query().map(|q| q.as_str().to_string()),
        Some("gpt".to_string())
    );
    assert_eq!(out.filter().date_range(), &DateRangeFilter::Last90Days);
}

// ===== TP-C*: ClearAll =====

/// spec.md#tp-clear-all TP-C1 — 全 filter リセット (C-UF4)
#[test]
fn tp_c1_clear_all_resets_filter_to_initial() {
    let mut feed = empty_feed();
    feed = UpdateFeedFilterUseCase::apply(
        feed,
        UpdateFeedFilterCommand::SetQuery {
            raw: "gpt".to_string(),
        },
    );
    feed = UpdateFeedFilterUseCase::apply(
        feed,
        UpdateFeedFilterCommand::SetTag {
            tag: Some(tag("coding")),
        },
    );
    feed = UpdateFeedFilterUseCase::apply(
        feed,
        UpdateFeedFilterCommand::SetDateRange {
            range: DateRangeFilter::Last7Days,
        },
    );

    let out = UpdateFeedFilterUseCase::apply(feed, UpdateFeedFilterCommand::ClearAll);

    assert!(out.filter().query().is_none());
    assert_eq!(out.filter().tag(), None);
    assert_eq!(out.filter().date_range(), &DateRangeFilter::All);
}

/// spec.md#tp-clear-all TP-C2 — ClearAll 2 回適用同値
#[test]
fn tp_c2_clear_all_is_idempotent() {
    let first = UpdateFeedFilterUseCase::apply(empty_feed(), UpdateFeedFilterCommand::ClearAll);
    let second = UpdateFeedFilterUseCase::apply(first.clone(), UpdateFeedFilterCommand::ClearAll);
    assert_eq!(first, second);
}

// ===== TP-S8-*: S8 シナリオ walkthrough =====

/// spec.md#tp-s8 TP-S8-1 — Given empty, When SetQuery "gpt", Then filter.query = "gpt"
#[test]
fn tp_s8_1_halfwidth_query_through_walkthrough() {
    let out = UpdateFeedFilterUseCase::apply(
        empty_feed(),
        UpdateFeedFilterCommand::SetQuery {
            raw: "gpt".to_string(),
        },
    );
    assert_eq!(out.filter().query().unwrap().as_str(), "gpt");
}

/// spec.md#tp-s8 TP-S8-2 — Given empty, When SetQuery "Ｇｐｔ", Then filter.query = "gpt"
#[test]
fn tp_s8_2_fullwidth_query_through_walkthrough() {
    let out = UpdateFeedFilterUseCase::apply(
        empty_feed(),
        UpdateFeedFilterCommand::SetQuery {
            raw: "Ｇｐｔ".to_string(),
        },
    );
    assert_eq!(out.filter().query().unwrap().as_str(), "gpt");
}

// TP-S8-3 (event 発行なし) は TP-SE1 で型レベルに保証されるため separate test 不要。

// ===== TP-I*: 不変条件 =====

proptest! {
    /// spec.md#tp-invariants TP-I1 — SetQuery 出力の query は None または NFC + lowercase (property)
    ///
    /// 任意の文字列入力に対し、結果の `query.as_str()` は以下のいずれか:
    /// (a) None (trim 後 empty)
    /// (b) Some(s) で `s == nfc(s).to_lowercase()` (再正規化しても変化しない、I-F1)
    #[test]
    fn tp_i1_query_is_always_normalized_or_none(raw in ".*") {
        let out = UpdateFeedFilterUseCase::apply(
            empty_feed(),
            UpdateFeedFilterCommand::SetQuery { raw: raw.clone() },
        );
        match out.filter().query() {
            None => prop_assert!(raw.trim().is_empty(), "None は trim 後 empty のときのみ ({:?})", raw),
            Some(q) => {
                use unicode_normalization::UnicodeNormalization;
                let recheck: String = q.as_str().nfkc().collect::<String>().to_lowercase();
                prop_assert_eq!(q.as_str(), recheck.as_str(), "再正規化で変化しない: {:?}", raw);
                prop_assert!(!q.as_str().is_empty(), "Some の中身は非空");
            }
        }
    }
}

/// spec.md#tp-invariants TP-I6 — ClearAll 後の filter は I-F6 初期状態
#[test]
fn tp_i6_clear_all_matches_initial_filter() {
    let initial = FeedFilter::initial();
    let out = UpdateFeedFilterUseCase::apply(empty_feed(), UpdateFeedFilterCommand::ClearAll);
    assert_eq!(out.filter(), &initial);
}

// ===== TP-SE*: 副作用ゼロ (type-level) =====

/// spec.md#tp-side-effects TP-SE1 — apply シグネチャに Repository / Bus 引数なし
///
/// 関数ポインタ型として固定。Repository / Bus を取る形に変更されると型不一致でこの test が落ちる。
#[test]
fn tp_se1_apply_has_pure_signature() {
    let _: fn(NoteFeed, UpdateFeedFilterCommand) -> NoteFeed = UpdateFeedFilterUseCase::apply;
}
