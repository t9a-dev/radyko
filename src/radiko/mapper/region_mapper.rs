use crate::{
    domain::{
        Logo,
        region::{Region, RegionStation, RegionStations},
    },
    radiko::dto::xml::region_xml::{RegionStationXml, RegionStationsXml, RegionXml},
};

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
