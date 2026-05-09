use std::str::FromStr;

use anyhow::bail;
use jiff::{Span, ToSpan, Zoned};
use jiff_cron::Schedule;
use thiserror::Error;

use crate::app::{
    config::{RadykoConfigKeywords, RadykoConfigRules},
    types::Station,
    utils::Utils,
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
    pub station: Station,
    pub selector: Selector,
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
