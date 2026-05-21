use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct LogoXml {
    #[serde(rename = "@width")]
    pub width: u32,
    #[serde(rename = "@height")]
    pub height: u32,
    #[serde(rename = "@align")]
    pub align: String,
    #[serde(rename = "$text")]
    pub url: String,
}
