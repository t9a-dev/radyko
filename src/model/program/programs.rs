use std::{cmp::Reverse, collections::HashMap};

use futures::{StreamExt, TryStreamExt, stream};

use crate::{
    RADYKO_CONCURRENCY,
    app::program_selector::ProgramSelector,
    model::program::{
        error::ProgramParseError,
        program::Program,
        program_id::{ProgramId, StartAt},
    },
    radiko::{
        RadikoClient,
        dto::{json::program_json::RootJson, xml::program_xml::RadikoProgramXml},
        jst_datetime::RadykoDateTime,
    },
};

#[derive(Debug, Clone)]
pub struct Programs {
    data: Vec<Program>,
}

impl Programs {
    pub fn to_vec(&self) -> Vec<Program> {
        self.data.clone()
    }

    pub fn find_program(self, start_at: &StartAt) -> Option<Program> {
        self.data.into_iter().find(|p| p.start_at().eq(start_at))
    }

    pub async fn resolve_program_ids(
        radiko_client: &RadikoClient,
        ids: Vec<ProgramId>,
    ) -> anyhow::Result<Vec<Program>> {
        let programs = stream::iter(ids)
            .map(|id| {
                let radiko_client = radiko_client.clone();
                async move {
                    radiko_client
                        .find_program(id.start_at(), id.station_id())
                        .await
                }
            })
            .buffer_unordered(RADYKO_CONCURRENCY)
            .try_filter_map(|program| async move { Ok(program) })
            .try_collect::<Vec<_>>()
            .await?;

        Ok(programs)
    }

    pub async fn resolve_selectors(
        radiko_client: &RadikoClient,
        selectors: Vec<ProgramSelector>,
    ) -> anyhow::Result<Vec<Program>> {
        let mut programs = futures::stream::iter(selectors)
            .map(|selector| async move { selector.resolve(radiko_client).await })
            .buffer_unordered(RADYKO_CONCURRENCY)
            .try_fold(Vec::new(), |mut result, programs| async move {
                result.extend(programs);
                Ok(result)
            })
            .await?;
        programs.sort_by_key(|p| Reverse(p.program_id().start_at().clone().date()));

        Ok(programs)
    }

    pub async fn start_time_to_programs(
        radiko_client: &RadikoClient,
        station_id: &str,
    ) -> anyhow::Result<HashMap<StartAt, Program>> {
        Ok(radiko_client
            .weekly_programs(station_id)
            .await?
            .data
            .into_iter()
            .map(|program| program.start_time_to_program())
            .collect::<HashMap<_, _>>())
    }
}

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
        Ok(Programs { data: programs })
    }
}

impl TryFrom<RootJson> for Programs {
    type Error = ProgramParseError;

    fn try_from(value: RootJson) -> Result<Self, Self::Error> {
        let Some(programs_json) = value.data else {
            return Ok(Self { data: vec![] });
        };

        Ok(Self {
            data: programs_json
                .into_iter()
                .flat_map(Program::try_from)
                .collect::<Vec<_>>(),
        })
    }
}
