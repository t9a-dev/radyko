use crate::{
    model::{
        logo::Logo,
        region::{Region, RegionStation, RegionStations},
        station::{Station, Stations},
    },
    radiko::dto::xml::{
        logo_xml::LogoXml,
        region_xml::{RegionStationXml, RegionStationsXml, RegionXml},
        station_xml::{StationXml, StationsXml},
    },
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
