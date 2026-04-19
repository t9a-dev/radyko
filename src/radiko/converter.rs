use anyhow::Context;
use chrono::{NaiveDateTime, TimeZone};
use chrono_tz::Asia::Tokyo;

use crate::{
    model::{
        logo::Logo,
        program::{Program, Programs},
        region::{Region, RegionStation, RegionStations},
        station::{Station, Stations},
    },
    radiko::xml::{
        logo::LogoXml,
        program::{ProgramXml, RadikoProgramXml},
        region::{RegionStationXml, RegionStationsXml, RegionXml},
        station::{StationXml, StationsXml},
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

impl From<RadikoProgramXml> for Programs {
    fn from(value: RadikoProgramXml) -> Self {
        let mut programs = Vec::new();
        for station in value.stations.station {
            for programs_xml in station.programs {
                let Some(programs_xml) = programs_xml.program else {
                    continue;
                };

                for program_xml in programs_xml {
                    let mut program = Program::from(program_xml);
                    program.station_id = station.id.clone();
                    programs.push(program);
                }
            }
        }
        Programs { data: programs }
    }
}

impl From<ProgramXml> for Program {
    fn from(value: ProgramXml) -> Self {
        let ft = Tokyo
            .from_local_datetime(
                &NaiveDateTime::parse_from_str(&value.ft, "%Y%m%d%H%M%S")
                    .with_context(|| format!("parse error ft_value: {}", value.ft))
                    .unwrap(),
            )
            .unwrap();
        let to = Tokyo
            .from_local_datetime(
                &NaiveDateTime::parse_from_str(&value.to, "%Y%m%d%H%M%S")
                    .with_context(|| format!("parse error to_value: {}", value.to))
                    .unwrap(),
            )
            .unwrap();
        Program {
            start_time: ft,
            end_time: to,
            station_id: "".to_string(),
            title: value.title.clone(),
            performer: value.pfm.unwrap_or_default(),
            // start_time_s: value.ftl.clone(),
            // end_time_s: value.tol.clone(),
            // info: value.info.unwrap_or_default(),
            // description: value.desc.unwrap_or_default(),
            // img: value.img.unwrap_or_default(),
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

pub mod jst_datetime {
    /// https://serde.rs/custom-date-format.html
    use chrono::{DateTime, NaiveDateTime, TimeZone};
    use chrono_tz::{Asia::Tokyo, Tz};
    use serde::{self, Deserialize, Deserializer, Serializer};

    const FORMAT: &str = "%Y-%m-%d %H:%M:%S";

    // The signature of a serialize_with function must follow the pattern:
    //
    //    fn serialize<S>(&T, S) -> Result<S::Ok, S::Error>
    //    where
    //        S: Serializer
    //
    // although it may also be generic over the input types T.
    pub fn serialize<S>(date: &DateTime<Tz>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(FORMAT));
        serializer.serialize_str(&s)
    }

    // The signature of a deserialize_with function must follow the pattern:
    //
    //    fn deserialize<'de, D>(D) -> Result<T, D::Error>
    //    where
    //        D: Deserializer<'de>
    //
    // although it may also be generic over the output types T.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Tz>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let dt = NaiveDateTime::parse_from_str(&s, FORMAT).unwrap();
        Ok(Tokyo.from_local_datetime(&dt).unwrap())
    }
}
