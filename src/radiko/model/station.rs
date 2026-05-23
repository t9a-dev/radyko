use serde_derive::Deserialize;

use crate::radiko::dto::xml::{StationXml, StationsXml};

use super::logo::Logo;

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct Stations {
    area_id: String,
    area_name: String,
    data: Vec<Station>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct Station {
    id: String,
    name: String,
    ascii_name: String,
    ruby: String,
    areafree: bool,
    timefree: bool,
    logos: Vec<Logo>,
    banner: String,
    href: String,
    simul_max_delay: u32,
    tf_max_delay: u32,
}

impl From<StationXml> for Station {
    fn from(value: StationXml) -> Self {
        Station {
            id: value.id,
            name: value.name,
            ascii_name: value.ascii_name,
            ruby: value.ruby,
            areafree: value.areafree == 1,
            timefree: value.timefree == 1,
            logos: value.logos.into_iter().map(Logo::from).collect(),
            banner: value.banner,
            href: value.href,
            simul_max_delay: value.simul_max_delay,
            tf_max_delay: value.tf_max_delay,
        }
    }
}

impl From<StationsXml> for Stations {
    fn from(value: StationsXml) -> Self {
        Stations {
            area_id: value.area_id,
            area_name: value.area_name,
            data: value.stations.into_iter().map(Station::from).collect(),
        }
    }
}
