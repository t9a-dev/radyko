use std::{collections::HashMap, str::FromStr, sync::Arc};

use anyhow::bail;
use jiff::{Span, ToSpan, Zoned};
use jiff_cron::Schedule;
use thiserror::Error;
use tracing::warn;

use crate::{
    app::{types::Station, utils::Utils},
    radiko::model::program::{Program, Programs, RadykoDateTime, StartAt},
};

use crate::app::ports::RadikoClient;

#[derive(Debug, Error, PartialEq)]
pub enum ScheduleError {
    #[error("invalid cron : {}",.0)]
    InvalidCron(String),
}

pub struct StartTimes(Vec<StartAt>);

impl StartTimes {
    fn from_cron(cron: String, days: Span, now: Option<Zoned>) -> anyhow::Result<Self> {
        let schedule = Schedule::from_str(&cron).map_err(|_| ScheduleError::InvalidCron(cron))?;
        let target_datetime = now.unwrap_or(Utils::now_in_tz_tokyo());
        let Ok(days_after) = target_datetime.checked_add(days) else {
            bail!(
                "failed calculate target_datetime after days. target_datetime: {:#?}, days: {:#?} ",
                target_datetime,
                days
            );
        };

        Ok(Self(
            schedule
                .after(&target_datetime)
                .take_while(|datetime| *datetime < days_after)
                .map(StartAt::new)
                .collect::<Vec<_>>(),
        ))
    }

    fn resolve_programs(self, start_time_to_programs: HashMap<StartAt, Program>) -> Vec<Program> {
        let mut programs = Vec::new();
        for start_time in self.0 {
            start_time_to_programs
                .get(&start_time)
                .inspect(|&program| programs.push(program.clone()));
        }
        programs
    }
}
pub struct Keywords(Vec<String>);

impl Keywords {
    async fn resolve_programs(
        self,
        radiko_client: Arc<dyn RadikoClient>,
        station: Station,
    ) -> anyhow::Result<Vec<Program>> {
        let mut programs = Vec::new();

        for keyword in self.0 {
            let result = match station {
                Station::Nationwide => radiko_client.search_programs(keyword, None).await?,
                Station::Id(ref station_id) => {
                    radiko_client
                        .search_programs(keyword, Some(station_id.as_str()))
                        .await?
                }
            };
            programs.push(result.to_vec());
        }

        Ok(programs.into_iter().flatten().collect::<Vec<_>>())
    }
}

pub enum Selector {
    StartTimes(StartTimes),
    Keywords(Keywords),
}

pub struct ProgramSelector {
    station: Station,
    selector: Selector,
}

impl ProgramSelector {
    pub fn new_rule_selector(
        station_id: Station,
        cron: String,
        now: Option<Zoned>,
    ) -> anyhow::Result<Self> {
        // radikoの番組表は1週間先までなので、Daysは7日固定
        Ok(Self {
            station: station_id,
            selector: Selector::StartTimes(StartTimes::from_cron(cron, 7.days(), now)?),
        })
    }

    pub fn new_keyword_selector(station_id: Station, keywords: Vec<String>) -> Self {
        Self {
            station: station_id,
            selector: Selector::Keywords(Keywords(keywords)),
        }
    }

    pub async fn resolve(
        self,
        radiko_client: Arc<dyn RadikoClient>,
    ) -> anyhow::Result<Vec<Program>> {
        match self.station {
            Station::Nationwide => match self.selector {
                Selector::Keywords(keywords) => Ok(keywords
                    .resolve_programs(radiko_client, self.station)
                    .await?),
                Selector::StartTimes(_) => {
                    warn!("指定した時間から始まる全ての放送を録音するようなユースケースには非対応");
                    Ok(vec![])
                }
            },
            Station::Id(ref station_id) => match self.selector {
                Selector::StartTimes(start_times) => {
                    let start_time_to_programs =
                        Programs::start_time_to_programs(radiko_client, station_id).await?;
                    Ok(start_times.resolve_programs(start_time_to_programs))
                }
                Selector::Keywords(keywords) => Ok(keywords
                    .resolve_programs(radiko_client, self.station)
                    .await?),
            },
        }
    }
}
