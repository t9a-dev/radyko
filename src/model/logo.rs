use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Logo {
    pub width: u32,
    pub height: u32,
    pub align: String,
    pub url: String,
}
