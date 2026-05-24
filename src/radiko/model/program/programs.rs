use std::{cmp::Reverse, collections::HashMap, sync::Arc};

use futures::{StreamExt, TryStreamExt, stream};

use crate::{
    RADYKO_CONCURRENCY,
    app::program_selector::ProgramSelector,
    radiko::dto::{json::RootJson, xml::RadikoProgramXml},
    radiko::model::program::{
        Program, ProgramParseError, RadykoDateTime, {ProgramId, StartAt},
    },
};

use crate::app::ports::RadikoClient;

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
        radiko_client: Arc<dyn RadikoClient>,
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
        radiko_client: Arc<dyn RadikoClient>,
        selectors: Vec<ProgramSelector>,
    ) -> anyhow::Result<Vec<Program>> {
        let mut programs = futures::stream::iter(selectors)
            .map(|selector| {
                let shared_radiko_client = Arc::clone(&radiko_client);
                async move { selector.resolve(shared_radiko_client).await }
            })
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
        radiko_client: Arc<dyn RadikoClient>,
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
