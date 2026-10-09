use serde::Serialize;
use specta::Type;

#[derive(Debug, Clone, Serialize, Type)]
pub struct DomainEvent<TPayload>
where
    TPayload: Serialize + Type + Clone,
{
    pub name: String,
    pub occurred_at: String,
    pub payload: TPayload,
}
