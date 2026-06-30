//! Tests for slice `list-feed` (spec.md#test-perspectives).
//!
//! 4 layer:
//! 1. NoteRepository::list_all 契約 (TP-LA*)
//! 2. NoteFeed::visible_notes pipeline: filter / sort / hydrate / S12 walkthrough (TP-F*, TP-S*, TP-V*, TP-S12-*)
//! 3. ListFeedUseCase::execute シグネチャ + 副作用ゼロ (TP-SE1)

use std::fs;

use time::macros::datetime;
use time::OffsetDateTime;

use crate::note_capture::shared::ports::NoteRepository;
use crate::note_capture::shared::types::{Note, NoteBody, Tag, TagSet, Timestamp};
use crate::note_capture::slices::create_note::infrastructure::FsNoteRepository;
use crate::note_feed::shared::types::{DateRangeFilter, FeedFilter, NormalizedQuery, NoteFeed};
use crate::user_preferences::shared::types::{SortDirection, SortField, SortOrder};

use super::application::{visible_notes_snapshot, ListFeedUseCase};
use super::domain::ListFeedCommand;

fn ts(dt: OffsetDateTime) -> Timestamp {
    Timestamp::from_offset_datetime(dt)
}

fn note(id_dt: OffsetDateTime, body: &str, tags: &[&str], updated_dt: OffsetDateTime) -> Note {
    let body = NoteBody::new(body.to_string()).expect("test body must be valid");
    let tags: Vec<Tag> = tags
        .iter()
        .map(|s| Tag::new(s).expect("test tag valid"))
        .collect();
    Note::from_persisted(body, TagSet::from_tags(tags), ts(id_dt), ts(updated_dt))
}

fn write_md(
    dir: &std::path::Path,
    id: &str,
    body: &str,
    tags_inline: &str,
    created_at: &str,
    updated_at: &str,
) {
    let content = format!(
        "---\ncreatedAt: {created}\nupdatedAt: {updated}\ntags: [{tags}]\n---\n{body}",
        created = created_at,
        updated = updated_at,
        tags = tags_inline,
        body = body,
    );
    fs::write(dir.join(format!("{id}.md")), content).unwrap();
}

// ===== TP-LA*: NoteRepository::list_all =====

/// TP-LA1 — 空 storage_dir → Vec::new()
#[test]
fn tp_la1_empty_storage_dir_yields_empty_vec() {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = FsNoteRepository::new(dir.path().to_path_buf());
    let notes = repo.list_all().expect("list_all ok");
    assert!(notes.is_empty());
}

/// TP-LA1b — 不在 storage_dir → Vec::new() (NotFound は skip、C-LF1)
#[test]
fn tp_la1b_missing_storage_dir_yields_empty_vec() {
    let dir = tempfile::tempdir().expect("tempdir");
    let nonexist = dir.path().join("does/not/exist");
    let repo = FsNoteRepository::new(nonexist);
    let notes = repo.list_all().expect("list_all ok");
    assert!(notes.is_empty());
}

/// TP-LA2 — 2 件の valid .md → 2 件の Note
#[test]
fn tp_la2_two_valid_notes_are_returned() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_md(
        dir.path(),
        "20260601100000",
        "hello",
        "rust, gpt",
        "20260601100000",
        "20260601100000",
    );
    write_md(
        dir.path(),
        "20260601100100",
        "world",
        "",
        "20260601100100",
        "20260601100100",
    );
    let repo = FsNoteRepository::new(dir.path().to_path_buf());
    let notes = repo.list_all().expect("list_all ok");
    assert_eq!(notes.len(), 2);
    let ids: Vec<_> = notes.iter().map(|n| n.id().as_str().to_string()).collect();
    assert!(ids.contains(&"20260601100000".to_string()));
    assert!(ids.contains(&"20260601100100".to_string()));
}

/// TP-LA3 — malformed が混在 → valid のみ、malformed は skip (C-LF1)
#[test]
fn tp_la3_malformed_files_are_skipped() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_md(
        dir.path(),
        "20260601100000",
        "ok",
        "",
        "20260601100000",
        "20260601100000",
    );
    fs::write(dir.path().join("20260601100100.md"), "not a frontmatter").unwrap();
    let repo = FsNoteRepository::new(dir.path().to_path_buf());
    let notes = repo.list_all().expect("list_all ok");
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].id().as_str(), "20260601100000");
}

/// TP-LA4 — .md 以外は無視
#[test]
fn tp_la4_non_md_files_are_ignored() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_md(
        dir.path(),
        "20260601100000",
        "ok",
        "",
        "20260601100000",
        "20260601100000",
    );
    fs::write(dir.path().join("README.txt"), "readme").unwrap();
    fs::write(dir.path().join("notes.json"), "{}").unwrap();
    let repo = FsNoteRepository::new(dir.path().to_path_buf());
    let notes = repo.list_all().expect("list_all ok");
    assert_eq!(notes.len(), 1);
}

// ===== TP-F*: filter 適用 =====

fn three_notes() -> Vec<Note> {
    vec![
        note(
            datetime!(2026-06-01 10:00 UTC),
            "first body about gpt",
            &["rust"],
            datetime!(2026-06-01 10:00 UTC),
        ),
        note(
            datetime!(2026-06-15 10:00 UTC),
            "second talks about other things",
            &["coding", "gpt"],
            datetime!(2026-06-15 10:00 UTC),
        ),
        note(
            datetime!(2026-06-26 10:00 UTC),
            "third unrelated",
            &[],
            datetime!(2026-06-26 10:00 UTC),
        ),
    ]
}

/// TP-F1 — filter 空 → source 全件
#[test]
fn tp_f1_empty_filter_returns_all_source() {
    let feed = NoteFeed::empty().hydrate(three_notes());
    let visible = feed.visible_notes();
    assert_eq!(visible.len(), 3);
}

/// TP-F2 — query "gpt" → body / tags に gpt を含むのみ
#[test]
fn tp_f2_query_matches_body_or_tags() {
    let filter = FeedFilter::initial().with_query(NormalizedQuery::from_raw("gpt"));
    let feed = NoteFeed::empty().hydrate(three_notes()).with_filter(filter);
    let visible = feed.visible_notes();
    assert_eq!(visible.len(), 2);
}

/// TP-F3 — 全角 "Ｇｐｔ" 入力 → NFKC + lowercase で "gpt" となり、body/tag の "gpt" にヒット
#[test]
fn tp_f3_fullwidth_query_matches_halfwidth() {
    let filter = FeedFilter::initial().with_query(NormalizedQuery::from_raw("Ｇｐｔ"));
    let feed = NoteFeed::empty().hydrate(three_notes()).with_filter(filter);
    let visible = feed.visible_notes();
    assert_eq!(visible.len(), 2);
}

/// TP-F4 — tag = Some("coding") → tags に coding を含むのみ
#[test]
fn tp_f4_tag_filter() {
    let tag = Tag::new("coding").unwrap();
    let filter = FeedFilter::initial().with_tag(Some(tag));
    let feed = NoteFeed::empty().hydrate(three_notes()).with_filter(filter);
    let visible = feed.visible_notes();
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].id().as_str(), "20260615100000");
}

/// TP-F5 — Last7Days → 直近 7 日以内のみ (now = 2026-06-27 12:00)
#[test]
fn tp_f5_last_7_days() {
    let filter = FeedFilter::initial().with_date_range(DateRangeFilter::Last7Days);
    let feed = NoteFeed::empty().hydrate(three_notes()).with_filter(filter);
    let visible = feed.visible_notes();
    let ids: Vec<_> = visible
        .iter()
        .map(|n| n.id().as_str().to_string())
        .collect();
    // 2026-06-01 と 2026-06-15 は 7 日より前。2026-06-26 のみ通る。
    assert_eq!(ids, vec!["20260626100000".to_string()]);
}

/// TP-F6 — query AND tag AND date_range (I-F4)
#[test]
fn tp_f6_and_composition() {
    let tag = Tag::new("gpt").unwrap();
    let filter = FeedFilter::initial()
        .with_query(NormalizedQuery::from_raw("second"))
        .with_tag(Some(tag.clone()))
        .with_date_range(DateRangeFilter::Last30Days);
    let feed = NoteFeed::empty().hydrate(three_notes()).with_filter(filter);
    let visible = feed.visible_notes();
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].id().as_str(), "20260615100000");
}

/// TP-F7 — query = None → query 軸無効 (count 不変)
#[test]
fn tp_f7_no_query_axis() {
    let filter = FeedFilter::initial();
    let feed = NoteFeed::empty().hydrate(three_notes()).with_filter(filter);
    let visible = feed.visible_notes();
    assert_eq!(visible.len(), 3);
}

// ===== TP-S*: sort 適用 =====

/// TP-S1 — CreatedAt desc → 新しい順
#[test]
fn tp_s1_sort_created_at_desc() {
    let feed = NoteFeed::empty()
        .hydrate(three_notes())
        .change_sort(SortOrder::new(SortField::CreatedAt, SortDirection::Desc));
    let visible = feed.visible_notes();
    let ids: Vec<_> = visible
        .iter()
        .map(|n| n.id().as_str().to_string())
        .collect();
    assert_eq!(
        ids,
        vec![
            "20260626100000".to_string(),
            "20260615100000".to_string(),
            "20260601100000".to_string(),
        ]
    );
}

/// TP-S2 — CreatedAt asc → 古い順
#[test]
fn tp_s2_sort_created_at_asc() {
    let feed = NoteFeed::empty()
        .hydrate(three_notes())
        .change_sort(SortOrder::new(SortField::CreatedAt, SortDirection::Asc));
    let visible = feed.visible_notes();
    let ids: Vec<_> = visible
        .iter()
        .map(|n| n.id().as_str().to_string())
        .collect();
    assert_eq!(
        ids,
        vec![
            "20260601100000".to_string(),
            "20260615100000".to_string(),
            "20260626100000".to_string(),
        ]
    );
}

/// TP-S3 — UpdatedAt desc
#[test]
fn tp_s3_sort_updated_at_desc() {
    let notes = vec![
        note(
            datetime!(2026-06-01 10:00 UTC),
            "a",
            &[],
            datetime!(2026-06-26 10:00 UTC),
        ),
        note(
            datetime!(2026-06-15 10:00 UTC),
            "b",
            &[],
            datetime!(2026-06-15 10:00 UTC),
        ),
    ];
    let feed = NoteFeed::empty()
        .hydrate(notes)
        .change_sort(SortOrder::new(SortField::UpdatedAt, SortDirection::Desc));
    let visible = feed.visible_notes();
    assert_eq!(visible[0].id().as_str(), "20260601100000");
    assert_eq!(visible[1].id().as_str(), "20260615100000");
}

/// TP-S4 — 同 sort key → id で tiebreak (I-F3、C-LF3)
#[test]
fn tp_s4_tiebreak_by_id() {
    let notes = vec![
        note(
            datetime!(2026-06-01 10:00 UTC),
            "a",
            &[],
            datetime!(2026-06-26 10:00 UTC),
        ),
        note(
            datetime!(2026-06-15 10:00 UTC),
            "b",
            &[],
            datetime!(2026-06-26 10:00 UTC),
        ),
    ];
    let feed = NoteFeed::empty()
        .hydrate(notes)
        .change_sort(SortOrder::new(SortField::UpdatedAt, SortDirection::Desc));
    let visible = feed.visible_notes();
    let ids: Vec<_> = visible
        .iter()
        .map(|n| n.id().as_str().to_string())
        .collect();
    assert_eq!(
        ids,
        vec!["20260615100000".to_string(), "20260601100000".to_string()],
        "updated_at 同値時は id 降順 (sort 方向と整合)"
    );
}

// ===== TP-V*: hydrate / visible =====

/// TP-V1 — hydrate 結果は source に反映
#[test]
fn tp_v1_hydrate_replaces_source() {
    let feed = NoteFeed::empty().hydrate(three_notes());
    assert_eq!(feed.source().len(), 3);
}

/// TP-V3 — 同じ notes で 2 回 hydrate → 結果同値 (C-LF9 冪等)
#[test]
fn tp_v3_hydrate_is_idempotent() {
    let first = NoteFeed::empty().hydrate(three_notes());
    let second = first.clone().hydrate(three_notes());
    assert_eq!(first, second);
}

// ===== TP-S12-*: S12 walkthrough =====

/// TP-S12-1 — Given 3 files、When 起動、Then visible_notes=3 (filter 空 + sort default)
#[test]
fn tp_s12_1_startup_lists_all_notes() {
    let feed = NoteFeed::empty().hydrate(three_notes());
    let visible = feed.visible_notes();
    assert_eq!(visible.len(), 3);
}

/// TP-S12-2 — Settings.sort_preference = {updated_at, asc} → visible_notes は updated_at 昇順
#[test]
fn tp_s12_2_startup_respects_sort_preference() {
    let feed = NoteFeed::empty()
        .hydrate(three_notes())
        .change_sort(SortOrder::new(SortField::UpdatedAt, SortDirection::Asc));
    let visible = feed.visible_notes();
    let ids: Vec<_> = visible
        .iter()
        .map(|n| n.id().as_str().to_string())
        .collect();
    assert_eq!(
        ids,
        vec![
            "20260601100000".to_string(),
            "20260615100000".to_string(),
            "20260626100000".to_string(),
        ]
    );
}

// ===== TP-SE1: シグネチャ =====

/// TP-SE1 — `ListFeedUseCase::execute` は Repository だけを inject、EventBus は取らない (C-LF6)
#[test]
fn tp_se1_execute_signature_takes_only_repository() {
    // 型レベル: `for<'a> fn(&'a ListFeedUseCase<R>, NoteFeed, ListFeedCommand) -> io::Result<NoteFeed>` を満たす
    fn _take<R: NoteRepository>(
        uc: &ListFeedUseCase<R>,
        feed: NoteFeed,
        cmd: ListFeedCommand,
    ) -> std::io::Result<NoteFeed> {
        uc.execute(feed, cmd)
    }
}

// ===== Integration: FsNoteRepository → ListFeedUseCase → visible_notes =====

#[test]
fn integration_fs_to_visible_notes() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_md(
        dir.path(),
        "20260620100000",
        "hello gpt",
        "rust",
        "20260620100000",
        "20260620100000",
    );
    write_md(
        dir.path(),
        "20260621100000",
        "unrelated",
        "coding",
        "20260621100000",
        "20260621100000",
    );

    let repo = FsNoteRepository::new(dir.path().to_path_buf());
    let uc = ListFeedUseCase::new(repo);
    let feed = NoteFeed::empty();
    let hydrated = uc.execute(feed, ListFeedCommand).expect("ok");
    assert_eq!(hydrated.source().len(), 2);

    let filter = FeedFilter::initial().with_query(NormalizedQuery::from_raw("gpt"));
    let visible = visible_notes_snapshot(&hydrated.with_filter(filter));
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].id().as_str(), "20260620100000");
}
