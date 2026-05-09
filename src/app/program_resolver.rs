use std::collections::HashMap;

use futures::{StreamExt, TryStreamExt, stream};
use jiff::Zoned;
use tracing::warn;

use crate::{
    RADYKO_CONCURRENCY,
    app::{
        program_selector::{Keywords, ProgramSelector, Selector, StartTimes},
        types::Station,
    },
    model::program::{Program, ProgramId},
    radiko::RadikoClient,
};

pub async fn resolve_program_id(
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

pub async fn resolve_selector(
    radiko_client: &RadikoClient,
    selector: ProgramSelector,
) -> anyhow::Result<Vec<Program>> {
    match selector.station {
        Station::Nationwide => match selector.selector {
            Selector::Keywords(keywords) => {
                Ok(resolve_keywords(radiko_client, keywords, selector.station).await?)
            }
            Selector::StartTimes(_) => {
                warn!("指定した時間から始まる全ての放送を録音するようなユースケースには非対応");
                Ok(vec![])
            }
        },
        Station::Id(ref station_id) => match selector.selector {
            Selector::StartTimes(start_times) => {
                Ok(resolve_start_times(radiko_client, start_times, station_id).await?)
            }
            Selector::Keywords(keywords) => {
                Ok(resolve_keywords(radiko_client, keywords, selector.station).await?)
            }
        },
    }
}

async fn resolve_keywords(
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

async fn resolve_start_times(
    radiko_client: &RadikoClient,
    start_times: StartTimes,
    station_id: &str,
) -> anyhow::Result<Vec<Program>> {
    let start_time_to_program: HashMap<Zoned, Program> = radiko_client
        .weekly_programs(station_id)
        .await?
        .data
        .into_iter()
        .map(|program| (program.clone().start_time, program))
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
