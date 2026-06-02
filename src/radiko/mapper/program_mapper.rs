use jiff::civil::DateTime;

use crate::{
    RADYKO_TZ_NAME,
    domain::program::{
        EndAt, Program, ProgramId, ProgramParseError, Programs, RadykoDateTime, StartAt, StationId,
    },
    radiko::dto::{
        json::program_json::{ProgramJson, RootJson},
        xml::program_xml::{ProgramXml, RadikoProgramXml},
    },
};

impl TryFrom<RadikoProgramXml> for Programs {
    type Error = ProgramParseError;

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
        Ok(Programs::new(programs))
    }
}

impl TryFrom<RootJson> for Programs {
    type Error = ProgramParseError;

    fn try_from(value: RootJson) -> Result<Self, Self::Error> {
        let Some(programs_json) = value.data else {
            return Ok(Programs::new(vec![]));
        };

        Ok(Programs::new(
            programs_json
                .into_iter()
                .flat_map(Program::try_from)
                .collect::<Vec<_>>(),
        ))
    }
}
impl TryFrom<ProgramXml> for Program {
    type Error = ProgramParseError;

    fn try_from(value: ProgramXml) -> Result<Self, Self::Error> {
        const RADIKO_XML_DATETIME_FORMAT: &str = "%Y%m%d%H%M%S";
        let ft = DateTime::strptime(RADIKO_XML_DATETIME_FORMAT, &value.ft)
            .map_err(|e| {
                ProgramParseError::Invalid(format!("failed parse ft: {}, error: {e:#?}", value.ft))
            })?
            .in_tz(RADYKO_TZ_NAME)
            .map_err(|e| {
                ProgramParseError::Invalid(format!(
                    "failed convert to Zoned datetime time_zone_name: {RADYKO_TZ_NAME}, error: {e:#?}"
                ))
            })?;
        let to = DateTime::strptime(RADIKO_XML_DATETIME_FORMAT, &value.to)
            .map_err(|e| {
                ProgramParseError::Invalid(format!("failed parse to: {}, error: {e:#?}", value.to))
            })?
            .in_tz(RADYKO_TZ_NAME)
            .map_err(|e| {
                ProgramParseError::Invalid(format!(
                    "failed convert to Zoned datetime time_zone_name: {RADYKO_TZ_NAME}, error: {e:#?}"
                ))
            })?;

        Ok(Program::new(
            ProgramId::new(
                StationId::new(value.station_id),
                StartAt::new(ft),
                EndAt::new(to),
            ),
            value.title.clone(),
            value.pfm.unwrap_or_default(),
        ))
    }
}

impl TryFrom<ProgramJson> for Program {
    type Error = ProgramParseError;

    fn try_from(value: ProgramJson) -> Result<Self, Self::Error> {
        const RADIKO_JSON_DATETIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
        let ft = DateTime::strptime(RADIKO_JSON_DATETIME_FORMAT, &value.start_time)
            .map_err(|e| {
                ProgramParseError::Invalid(format!("failed parse ft: {}, error: {e:#?}", value.start_time))
            })?
            .in_tz(RADYKO_TZ_NAME)
            .map_err(|e| {
                ProgramParseError::Invalid(format!(
                    "failed convert to Zoned datetime time_zone_name: {RADYKO_TZ_NAME}, error: {e:#?}"
                ))
            })?;
        let to = DateTime::strptime(RADIKO_JSON_DATETIME_FORMAT, &value.end_time)
            .map_err(|e| {
                ProgramParseError::Invalid(format!("failed parse to: {}, error: {e:#?}", value.end_time))
            })?
            .in_tz(RADYKO_TZ_NAME)
            .map_err(|e| {
                ProgramParseError::Invalid(format!(
                    "failed convert to Zoned datetime time_zone_name: {RADYKO_TZ_NAME}, error: {e:#?}"
                ))
            })?;

        Ok(Program::new(
            ProgramId::new(
                StationId::new(value.station_id),
                StartAt::new(ft),
                EndAt::new(to),
            ),
            value.title,
            value.performer,
        ))
    }
}
