use std::{cmp::Reverse, collections::HashMap};

use futures::{StreamExt, TryStreamExt, stream};
use jiff::Zoned;
use serde::{Deserialize, Serialize};

use crate::{
    RADYKO_CONCURRENCY,
    app::{
        program_selector::{Keywords, ProgramSelector, StartTimes},
        types::Station,
    },
    model::program::{error::ProgramParseError, program::Program, program_id::ProgramId},
    radiko::{RadikoClient, dto::program_xml::RadikoProgramXml},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Programs {
    pub data: Vec<Program>,
}

impl Programs {
    pub fn find_program(self, start_at: Zoned) -> Option<Program> {
        self.data.into_iter().find(|p| p.start_time.eq(&start_at))
    }

    pub async fn resolve_program_ids(
        radiko_client: &RadikoClient,
        ids: Vec<ProgramId>,
    ) -> anyhow::Result<Vec<Program>> {
        let programs = stream::iter(ids.iter())
            .map(|id| {
                let radiko_client = radiko_client.clone();
                async move { radiko_client.find_program(id.clone().1.0, &id.0.0).await }
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
            .map(|selector| async move { selector.resolve_selector(radiko_client).await })
            .buffer_unordered(RADYKO_CONCURRENCY)
            .try_fold(Vec::new(), |mut result, programs| async move {
                result.extend(programs);
                Ok(result)
            })
            .await?;
        programs.sort_by_key(|p| Reverse(p.start_time.clone()));

        Ok(programs)
    }

    pub async fn resolve_keywords(
        radiko_client: &RadikoClient,
        keywords: Keywords,
        station: Station,
    ) -> anyhow::Result<Vec<Program>> {
        let mut programs = Vec::new();
        let Keywords(keywords) = keywords;

        for keyword in keywords {
            let result = match station {
                Station::Nationwide => radiko_client.search_programs(keyword, None).await?,
                Station::Id(ref station_id) => {
                    radiko_client
                        .search_programs(keyword, Some(station_id.as_str()))
                        .await?
                }
            };
            programs.push(result.data);
        }

        Ok(programs.into_iter().flatten().collect::<Vec<_>>())
    }

    pub async fn resolve_start_times(
        radiko_client: &RadikoClient,
        start_times: StartTimes,
        station_id: &str,
    ) -> anyhow::Result<Vec<Program>> {
        let start_time_to_program: HashMap<Zoned, Program> = radiko_client
            .weekly_programs(station_id)
            .await?
            .data
            .into_iter()
            .map(|program| program.start_time_to_program())
            .collect();

        let mut programs = Vec::new();
        let StartTimes(start_times) = start_times;
        for start_time in start_times {
            start_time_to_program
                .get(&start_time)
                .inspect(|&program| programs.push(program.clone()));
        }

        Ok(programs)
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
