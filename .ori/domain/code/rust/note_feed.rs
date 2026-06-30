use crate::note::{Note, NoteId, Tag};
use serde::{Deserialize, Serialize};
use time::Date;
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedQuery(String);

impl NormalizedQuery {
    pub fn from_raw(raw: &str) -> Option<Self> {
        let normalized: String = raw.nfkc().collect::<String>().to_lowercase();
        if normalized.is_empty() {
            None
        } else {
            Some(Self(normalized))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DateRangeFilter {
    Last7Days,
    Last30Days,
    Last90Days,
    All,
    Custom { from: Date, to: Date },
}

impl Default for DateRangeFilter {
    fn default() -> Self {
        Self::All
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortField {
    CreatedAt,
    UpdatedAt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SortOrder {
    pub field: SortField,
    pub direction: SortDirection,
}

impl SortOrder {
    pub const fn default_value() -> Self {
        Self {
            field: SortField::CreatedAt,
            direction: SortDirection::Desc,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FeedFilter {
    pub query: Option<NormalizedQuery>,
    pub date_range: DateRangeFilter,
    pub tag: Option<Tag>,
}

#[derive(Debug, Clone)]
pub struct NoteFeed {
    source: Vec<Note>,
    filter: FeedFilter,
    sort: SortOrder,
}

impl NoteFeed {
    pub fn empty() -> Self {
        Self {
            source: Vec::new(),
            filter: FeedFilter::default(),
            sort: SortOrder::default_value(),
        }
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

    pub fn hydrate(mut self, notes: Vec<Note>) -> Self {
        self.source = notes;
        self
    }

    pub fn upsert_note(mut self, note: Note) -> Self {
        if let Some(existing) = self
            .source
            .iter_mut()
            .find(|n| n.id() == note.id())
        {
            *existing = note;
        } else {
            self.source.push(note);
        }
        self
    }

    pub fn remove_note(mut self, note_id: &NoteId) -> Self {
        self.source.retain(|n| n.id() != note_id);
        self
    }

    pub fn set_query(mut self, raw: &str) -> Self {
        self.filter.query = NormalizedQuery::from_raw(raw);
        self
    }

    pub fn set_date_range(mut self, r: DateRangeFilter) -> Self {
        self.filter.date_range = r;
        self
    }

    pub fn set_tag(mut self, tag: Option<Tag>) -> Self {
        self.filter.tag = tag;
        self
    }

    pub fn clear_filters(mut self) -> Self {
        self.filter = FeedFilter::default();
        self
    }

    pub fn change_sort(mut self, s: SortOrder) -> Self {
        self.sort = s;
        self
    }

    pub fn visible_notes(&self) -> Vec<&Note> {
        let mut filtered: Vec<&Note> = self
            .source
            .iter()
            .filter(|n| self.matches(n))
            .collect();
        filtered.sort_by(|a, b| {
            let key_a = self.sort_key(a);
            let key_b = self.sort_key(b);
            match self.sort.direction {
                SortDirection::Asc => key_a.cmp(&key_b),
                SortDirection::Desc => key_b.cmp(&key_a),
            }
        });
        filtered
    }

    fn matches(&self, note: &Note) -> bool {
        if let Some(t) = &self.filter.tag {
            if !note.tags().contains_name(t.as_str()) {
                return false;
            }
        }
        if let Some(q) = &self.filter.query {
            let body_norm: String = note
                .body()
                .as_str()
                .nfkc()
                .collect::<String>()
                .to_lowercase();
            let tag_match = note
                .tags()
                .as_slice()
                .iter()
                .any(|t| t.as_str().contains(q.as_str()));
            if !body_norm.contains(q.as_str()) && !tag_match {
                return false;
            }
        }
        // date_range filter is not implemented here; left as TODO for Phase 11.
        true
    }

    fn sort_key<'b>(&self, note: &'b Note) -> (time::OffsetDateTime, &'b str) {
        let ts = match self.sort.field {
            SortField::CreatedAt => note.created_at().inner(),
            SortField::UpdatedAt => note.updated_at().inner(),
        };
        (ts, note.id().as_str())
    }
}
