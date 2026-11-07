use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Event {
    pub id: String,
    pub title: String,
    pub description: String,
    pub starts_at: i64,
    pub reminder_minutes: u32,
    pub tags: Vec<String>,
    pub notification_id: i32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NewEvent {
    pub title: String,
    pub description: String,
    pub starts_at: i64,
    pub reminder_minutes: u32,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct AgendaData {
    pub events: Vec<Event>,
}
