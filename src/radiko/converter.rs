use jiff::civil::DateTime;
use thiserror::Error;

use crate::{
    RADYKO_TZ_NAME,
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

#[derive(Debug, Error)]
pub enum ConvertError {
    #[error("{}",.0)]
    Invalid(String),
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

impl TryFrom<RadikoProgramXml> for Programs {
    type Error = ConvertError;

    fn try_from(value: RadikoProgramXml) -> Result<Self, Self::Error> {
        let mut programs = Vec::new();
        for station in value.stations.station {
            for programs_xml in station.programs {
                let Some(programs_xml) = programs_xml.program else {
                    continue;
                };

                for mut program_xml in programs_xml {
                    program_xml.station_id = station.id.clone();
                    programs.push(Program::try_from(program_xml)?);
                }
            }
        }
        Ok(Programs { data: programs })
    }
}

impl TryFrom<ProgramXml> for Program {
    type Error = ConvertError;

    fn try_from(value: ProgramXml) -> Result<Self, Self::Error> {
        const FORMAT: &str = "%Y%m%d%H%M%S";
        let ft = DateTime::strptime(FORMAT, &value.ft)
            .map_err(|e| {
                ConvertError::Invalid(format!("failed parse ft: {}, error: {e:#?}", value.ft))
            })?
            .in_tz(RADYKO_TZ_NAME)
            .map_err(|e| {
                ConvertError::Invalid(format!(
                    "failed convert to Zoned datetime time_zone_name: {RADYKO_TZ_NAME}, error: {e:#?}"
                ))
            })?;
        let to = DateTime::strptime(FORMAT, &value.to)
            .map_err(|e| {
                ConvertError::Invalid(format!("failed parse to: {}, error: {e:#?}", value.to))
            })?
            .in_tz(RADYKO_TZ_NAME)
            .map_err(|e| {
                ConvertError::Invalid(format!(
                    "failed convert to Zoned datetime time_zone_name: {RADYKO_TZ_NAME}, error: {e:#?}"
                ))
            })?;

        Ok(Program {
            start_time: ft,
            end_time: to,
            station_id: value.station_id,
            title: value.title.clone(),
            performer: value.pfm.unwrap_or_default(),
        })
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

    use jiff::{Zoned, civil::DateTime};
    /// https://serde.rs/custom-date-format.html
    use serde::{self, Deserialize, Deserializer, Serializer};
    use tracing::error;

    use crate::RADYKO_TZ_NAME;

    const FORMAT: &str = "%Y-%m-%d %H:%M:%S";

    // The signature of a serialize_with function must follow the pattern:
    //
    //    fn serialize<S>(&T, S) -> Result<S::Ok, S::Error>
    //    where
    //        S: Serializer
    //
    // although it may also be generic over the input types T.
    pub fn serialize<S>(date: &Zoned, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.strftime(FORMAT));
        serializer.serialize_str(&s)
    }

    // The signature of a deserialize_with function must follow the pattern:
    //
    //    fn deserialize<'de, D>(D) -> Result<T, D::Error>
    //    where
    //        D: Deserializer<'de>
    //
    // although it may also be generic over the output types T.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Zoned, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(DateTime::strptime(FORMAT, &s)
            .unwrap()
            .in_tz(RADYKO_TZ_NAME)
            .map_err(|e| error!("jst_datetime deserialize error s: {s} error: {e:#?}"))
            .unwrap())
    }
}
