use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProfileKind {
    #[serde(rename = "HOST")]
    HOST,
    #[serde(rename = "LOCAL")]
    LOCAL,
    #[serde(rename = "GUEST")]
    GUEST,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerracottaProfile {
    #[serde(rename = "machine_id")]
    pub machine_id: String,
    pub name: String,
    pub vendor: String,
    pub kind: ProfileKind,
}