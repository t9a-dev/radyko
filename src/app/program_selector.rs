use std::str::FromStr;

use anyhow::bail;
use jiff::{Span, ToSpan, Zoned};
use jiff_cron::Schedule;
use thiserror::Error;
use tracing::warn;

use crate::{
    app::{
        config::{RadykoConfigKeywords, RadykoConfigRules},
        types::Station,
        utils::Utils,
    },
    model::program::{program::Program, programs::Programs},
    radiko::RadikoClient,
};

#[derive(Debug, Error, PartialEq)]
pub enum ScheduleError {
    #[error("invalid cron : {}",.0)]
    InvalidCron(String),
}

pub struct StartTimes(pub Vec<Zoned>);
pub struct Keywords(pub Vec<String>);
pub enum Selector {
    StartTimes(StartTimes),
    Keywords(Keywords),
}

pub struct ProgramSelector {
    station: Station,
    selector: Selector,
}

impl ProgramSelector {
    pub fn from_rules(rules: RadykoConfigRules) -> anyhow::Result<Vec<Self>> {
        Ok(rules
            .0
            .into_iter()
            .flat_map(|(station_id, cron_list)| {
                cron_list
                    .into_iter()
                    .flat_map(|cron| Self::new_rules(station_id.clone(), cron, None))
                    .collect::<Vec<Self>>()
            })
            .collect::<Vec<Self>>())
    }

    pub fn from_keywords(keyword_config: RadykoConfigKeywords) -> Vec<Self> {
        keyword_config
            .0
            .into_iter()
            .map(|(station_id, keywords)| Self::new_keywords(station_id, keywords))
            .collect()
    }

    pub async fn resolve_selector(
        self,
        radiko_client: &RadikoClient,
    ) -> anyhow::Result<Vec<Program>> {
        match self.station {
            Station::Nationwide => match self.selector {
                Selector::Keywords(keywords) => {
                    Ok(Programs::resolve_keywords(radiko_client, keywords, self.station).await?)
                }
                Selector::StartTimes(_) => {
                    warn!("指定した時間から始まる全ての放送を録音するようなユースケースには非対応");
                    Ok(vec![])
                }
            },
            Station::Id(ref station_id) => match self.selector {
                Selector::StartTimes(start_times) => {
                    Ok(
                        Programs::resolve_start_times(radiko_client, start_times, station_id)
                            .await?,
                    )
                }
                Selector::Keywords(keywords) => {
                    Ok(Programs::resolve_keywords(radiko_client, keywords, self.station).await?)
                }
            },
        }
    }

    fn new_rules(station_id: Station, cron: String, now: Option<Zoned>) -> anyhow::Result<Self> {
        // radikoの番組表は1週間先までなので、Daysは7日固定
        Ok(Self {
            station: station_id,
            selector: Selector::StartTimes(StartTimes(Self::start_datetimes_from_cron(
                cron,
                7.days(),
                now,
            )?)),
        })
    }

    fn new_keywords(station_id: Station, keywords: Vec<String>) -> Self {
        Self {
            station: station_id,
            selector: Selector::Keywords(Keywords(keywords)),
        }
    }

    fn start_datetimes_from_cron(
        cron: String,
        days: Span,
        now: Option<Zoned>,
    ) -> anyhow::Result<Vec<Zoned>> {
        let schedule = Schedule::from_str(&cron).map_err(|_| ScheduleError::InvalidCron(cron))?;
        let target_datetime = now.unwrap_or(Utils::now_in_tz_tokyo());
        let Ok(days_after) = target_datetime.checked_add(days) else {
            bail!(
                "failed calculate target_datetime after days. target_datetime: {:#?}, days: {:#?} ",
                target_datetime,
                days
            );
        };

        Ok(schedule
            .after(&target_datetime)
            .take_while(|datetime| *datetime < days_after)
            .collect())
    }
}
