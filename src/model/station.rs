use serde_derive::{Deserialize, Serialize};

use super::logo::Logo;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stations {
    pub area_id: String,
    pub area_name: String,
    pub data: Vec<Station>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Station {
    pub id: String,
    pub name: String,
    pub ascii_name: String,
    pub ruby: String,
    pub areafree: bool,
    pub timefree: bool,
    pub logos: Vec<Logo>,
    pub banner: String,
    pub href: String,
    pub simul_max_delay: u32,
    pub tf_max_delay: u32,
}
