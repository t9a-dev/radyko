use crate::{domain::Logo, radiko::dto::xml::logo_xml::LogoXml};

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
