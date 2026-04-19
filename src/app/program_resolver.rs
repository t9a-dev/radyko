use std::collections::HashMap;

use chrono::DateTime;
use chrono_tz::Tz;

use tracing::warn;

use crate::{
    app::{
        program_selector::{Keywords, ProgramSelector, Selector, StartTimes},
        types::Station,
    },
    model::program::Program,
    radiko::RadikoClient,
};

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
    let start_time_to_program: HashMap<DateTime<Tz>, Program> = radiko_client
        .weekly_programs(station_id)
        .await?
        .data
        .into_iter()
        .map(|program| (program.start_time, program))
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
