use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Category {
    Create,
    Learn,
    Explore,
    Move,
    Life,
    People,
    Recover,
    Think,
    Unknown,
}

impl Category {
    pub const ALL: [Category; 9] = [
        Self::Create,
        Self::Learn,
        Self::Explore,
        Self::Move,
        Self::Life,
        Self::People,
        Self::Recover,
        Self::Think,
        Self::Unknown,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Create => "Create",
            Self::Learn => "Learn",
            Self::Explore => "Explore",
            Self::Move => "Move",
            Self::Life => "Life",
            Self::People => "People",
            Self::Recover => "Recover",
            Self::Think => "Think",
            Self::Unknown => "Unknown",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.as_str() == s)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub active_minutes: f64,
    pub idle_minutes: f64,
    pub context_switches: u32,
    pub category: Category,
    pub application_ids: Vec<String>,
    pub project_id: Option<String>,
}
