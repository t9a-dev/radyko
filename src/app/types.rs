#[derive(Debug, PartialEq, Eq)]
pub struct Keyword(pub String);

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
/// radikoにおける放送局のId. TBS,LFRなど
pub enum Station {
    /// 全国: 放送局指定なし
    Nationwide,
    Id(String),
}

mod station_serde {
    use super::*;
    use serde::{Deserialize, Deserializer, Serializer};

    const NATIONWIDE: &str = "nationwide";

    impl<'de> Deserialize<'de> for Station {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            let raw = String::deserialize(deserializer)?;

            if raw.eq_ignore_ascii_case(NATIONWIDE) {
                Ok(Station::Nationwide)
            } else {
                Ok(Station::Id(raw))
            }
        }
    }

    impl serde::Serialize for Station {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            match self {
                Station::Nationwide => serializer.serialize_str(NATIONWIDE),
                Station::Id(s) => serializer.serialize_str(s),
            }
        }
    }
}
