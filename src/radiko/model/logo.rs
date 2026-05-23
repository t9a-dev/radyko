use serde_derive::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Logo {
    pub width: u32,
    pub height: u32,
    pub align: String,
    pub url: String,
}
