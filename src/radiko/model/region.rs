use serde_derive::Deserialize;

use crate::radiko::dto::xml::{RegionStationXml, RegionStationsXml, RegionXml};

use super::logo::Logo;

#[derive(Debug, Deserialize, Clone)]
pub struct Region {
    pub stations_groups: Vec<RegionStations>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RegionStations {
    pub ascii_name: String,
    pub region_id: String,
    pub region_name: String,
    pub stations: Vec<RegionStation>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RegionStation {
    pub id: String,
    pub name: String,
    pub ascii_name: String,
    pub ruby: String,
    pub areafree: u8,
    pub timefree: u8,
    pub logos: Vec<Logo>,
    pub tf_max_delay: u32,
    pub banner: String,
    pub area_id: String,
    pub href: String,
    pub simul_max_delay: u32,
}

impl From<RegionXml> for Region {
    fn from(value: RegionXml) -> Self {
        Self {
            stations_groups: value
                .region_stations_groups
                .into_iter()
                .map(RegionStations::from)
                .collect(),
        }
    }
}

impl From<RegionStationXml> for RegionStation {
    fn from(value: RegionStationXml) -> Self {
        Self {
            id: value.id,
            name: value.name,
            ascii_name: value.ascii_name,
            ruby: value.ruby,
            areafree: value.areafree,
            timefree: value.timefree,
            logos: value.logos.into_iter().map(Logo::from).collect(),
            tf_max_delay: value.tf_max_delay,
            banner: value.banner,
            area_id: value.area_id,
            href: value.href,
            simul_max_delay: value.simul_max_delay,
        }
    }
}

impl From<RegionStationsXml> for RegionStations {
    fn from(value: RegionStationsXml) -> Self {
        Self {
            ascii_name: value.ascii_name,
            region_id: value.region_id,
            region_name: value.region_name,
            stations: value
                .stations
                .into_iter()
                .map(RegionStation::from)
                .collect(),
        }
    }
}
