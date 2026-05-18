use serde::{Deserialize, Serialize};

use super::logo_xml::LogoXml;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename = "region")]
pub struct RegionXml {
    #[serde(rename = "stations")]
    pub region_stations_groups: Vec<RegionStationsXml>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RegionStationsXml {
    #[serde(rename = "@ascii_name")]
    pub ascii_name: String,
    #[serde(rename = "@region_id")]
    pub region_id: String,
    #[serde(rename = "@region_name")]
    pub region_name: String,
    #[serde(rename = "station")]
    pub stations: Vec<RegionStationXml>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RegionStationXml {
    pub id: String,
    pub name: String,
    pub ascii_name: String,
    pub ruby: String,
    pub areafree: u8,
    pub timefree: u8,
    #[serde(rename = "logo")]
    pub logos: Vec<LogoXml>,
    pub tf_max_delay: u32,
    pub banner: String,
    pub area_id: String,
    pub href: String,
    pub simul_max_delay: u32,
}
