use time::{Duration, OffsetDateTime};
use unicode_normalization::UnicodeNormalization;

use crate::note_capture::shared::types::{Note, NoteId};
use crate::user_preferences::shared::types::{SortDirection, SortField, SortOrder};

use super::{DateRangeFilter, FeedFilter, NormalizedQuery};

/// Note Feed BC の唯一の集約 root (`aggregates.md#note-feed-aggregate`)。read model、揮発。
///
/// `update-feed-filter` slice では filter 軸のみを扱い、`source` / `sort` は drop していた。
/// `change-sort-order` slice で `sort: SortOrder` を、`list-feed` slice で `source: Vec<Note>` を
/// 復活させた (aggregates.md#note-feed-aggregate-elements、`source` の Vec<Note> 採用根拠は
/// `workflows/list-feed.md#notes`)。
///
/// `SortOrder` は `user_preferences::shared::types::SortOrder` を直接借りる
/// (Customer-Supplier 規約。Supplier = User Preferences、Customer = Note Feed)。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NoteFeed {
    source: Vec<Note>,
    filter: FeedFilter,
    sort: SortOrder,
}

impl NoteFeed {
    /// I-F6 の起動時初期状態 (source 空 + filter 空 + sort default = {CreatedAt, Desc})。
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn filter(&self) -> &FeedFilter {
        &self.filter
    }

    pub fn sort(&self) -> SortOrder {
        self.sort
    }

    pub fn source(&self) -> &[Note] {
        &self.source
    }

    /// FeedFilter を差し替えた新しい NoteFeed を返す (move semantics)。
    pub fn with_filter(mut self, filter: FeedFilter) -> Self {
        self.filter = filter;
        self
    }

    /// `aggregates.md#note-feed-aggregate-operations` の `change_sort`。
    /// in-memory 反映のみ。Settings 永続化は `change-sort-order` slice の application service
    /// が同一トランザクションで担う (`#notes-sort-side-effect`)。
    pub fn change_sort(mut self, sort: SortOrder) -> Self {
        self.sort = sort;
        self
    }

    /// `workflows/list-feed.md#steps` の `hydrateFeedSource`。
    /// `source` を差し替える pure 関数 (C-LF9 冪等性)。
    pub fn hydrate(mut self, notes: Vec<Note>) -> Self {
        self.source = notes;
        self
    }

    /// `aggregates.md#note-feed-aggregate-operations` の `upsert_note` (I-F8)。
    /// `source` 内の `note.id` と一致する要素があれば置換、なければ末尾に追加。
    /// 変更後も現在の filter / sort は維持される。
    pub fn upsert_note(mut self, note: Note) -> Self {
        let note_id = note.id();
        if let Some(existing) = self.source.iter_mut().find(|n| n.id() == note_id) {
            *existing = note;
        } else {
            self.source.push(note);
        }
        self
    }

    /// `aggregates.md#note-feed-aggregate-operations` の `remove_note` (I-F8)。
    /// `source` から `note_id` に一致する要素を削除。該当なしの場合は no-op。
    pub fn remove_note(mut self, note_id: &NoteId) -> Self {
        self.source.retain(|n| n.id() != note_id);
        self
    }

    /// `aggregates.md#note-feed-aggregate-queries` の `visible_notes`。
    /// filter を AND 合成 (I-F4) して、sort 適用後の `Vec<&Note>` を返す (C-LF2)。
    /// `DateRangeFilter::Last*Days` の評価には `OffsetDateTime::now_utc()` を内部で使用する
    /// (aggregates.md 改訂により `now` パラメータ削除)。
    pub fn visible_notes(&self) -> Vec<&Note> {
        let now = OffsetDateTime::now_utc();
        let mut filtered: Vec<&Note> = self
            .source
            .iter()
            .filter(|note| matches_filter(note, &self.filter, now))
            .collect();
        apply_sort(&mut filtered, self.sort);
        filtered
    }
}

/// I-F4 AND + early short-circuit (C-LF4)。query / tag / date_range の全軸を満たす Note のみ通す。
fn matches_filter(note: &Note, filter: &FeedFilter, now: OffsetDateTime) -> bool {
    if let Some(q) = filter.query() {
        if !matches_query(note, q) {
            return false;
        }
    }
    if let Some(tag) = filter.tag() {
        if !note
            .tags()
            .as_slice()
            .iter()
            .any(|t| t.name() == tag.name())
        {
            return false;
        }
    }
    matches_date_range(note, filter.date_range(), now)
}

/// I-F5: body + tags[*].name の substring (case-insensitive、NormalizedQuery は NFKC + lowercase 済)。C-LF5。
fn matches_query(note: &Note, query: &NormalizedQuery) -> bool {
    let needle = query.as_str();
    let body_lc: String = note
        .body()
        .as_str()
        .nfkc()
        .collect::<String>()
        .to_lowercase();
    if body_lc.contains(needle) {
        return true;
    }
    note.tags()
        .as_slice()
        .iter()
        .any(|t| t.name().contains(needle))
}

/// C-LF8: `Note.created_at` ベース。`Custom { from, to }` は `FeedDate` (day-precision) で
/// 型強化済み。`from > to` は `DateRangeFilter::custom` / `validate` で reject されるため、
/// 本関数に到達した `Custom` は `from <= to` を満たす前提で matching する。
fn matches_date_range(note: &Note, range: &DateRangeFilter, now: OffsetDateTime) -> bool {
    let note_dt = note.created_at().into_offset_datetime();
    match range {
        DateRangeFilter::All => true,
        DateRangeFilter::Last7Days => note_dt >= now - Duration::days(7),
        DateRangeFilter::Last30Days => note_dt >= now - Duration::days(30),
        DateRangeFilter::Last90Days => note_dt >= now - Duration::days(90),
        DateRangeFilter::Custom { from, to } => {
            let note_d = note_dt.date();
            note_d >= from.as_date() && note_d <= to.as_date()
        }
    }
}

/// C-LF3: stable sort + I-F3: 同 sort key は `id` (= created_at 秒精度) で tiebreak。
/// Rust の `sort_by` は stable sort なので、key が等しいときの順序は入力順を保つ。
/// 入力 (storage_dir の read_dir 順) を決定論にするため、ここで `id` で tiebreak する。
fn apply_sort(notes: &mut [&Note], sort: SortOrder) {
    notes.sort_by(|a, b| {
        let primary = compare_by_field(a, b, sort.field());
        let tiebreak = a.id().as_str().cmp(b.id().as_str());
        let combined = primary.then(tiebreak);
        if sort.direction() == SortDirection::Desc {
            combined.reverse()
        } else {
            combined
        }
    });
}

fn compare_by_field(a: &Note, b: &Note, field: SortField) -> std::cmp::Ordering {
    match field {
        SortField::CreatedAt => a.created_at().cmp(&b.created_at()),
        SortField::UpdatedAt => a.updated_at().cmp(&b.updated_at()),
    }
}
