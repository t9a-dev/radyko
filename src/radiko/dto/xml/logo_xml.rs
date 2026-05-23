use serde::Deserialize;

use crate::radiko::model::Logo;

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

impl From<LogoXml> for Logo {
    fn from(value: LogoXml) -> Self {
        Logo {
            width: value.width,
            height: value.height,
            align: value.align,
            url: value.url,
        }
    }
}
