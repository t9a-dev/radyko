use serde_derive::{Deserialize, Serialize};

use super::logo::Logo;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Region {
    pub stations_groups: Vec<RegionStations>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RegionStations {
    pub ascii_name: String,
    pub region_id: String,
    pub region_name: String,
    pub stations: Vec<RegionStation>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
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
