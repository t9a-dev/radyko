use std::str::FromStr;

use anyhow::bail;
use chrono::{DateTime, Days, Utc};
use chrono_tz::{Asia::Tokyo, Tz};
use cron::Schedule;
use thiserror::Error;

use crate::app::{
    config::{RadykoConfigKeywords, RadykoConfigRules},
    types::Station,
};

#[derive(Debug, Error, PartialEq)]
pub enum ScheduleError {
    #[error("invalid cron : {}",.0)]
    InvalidCron(String),
}

pub struct StartTimes(pub Vec<DateTime<Tz>>);
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

    fn new_rules(
        station_id: Station,
        cron: String,
        now: Option<DateTime<Tz>>,
    ) -> anyhow::Result<Self> {
        // radikoの番組表は1週間先までなので、Daysは7日固定
        Ok(Self {
            station: station_id,
            selector: Selector::StartTimes(StartTimes(Self::start_datetimes_from_cron(
                cron,
                Days::new(7),
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
        days: Days,
        now: Option<DateTime<Tz>>,
    ) -> anyhow::Result<Vec<DateTime<Tz>>> {
        let schedule = Schedule::from_str(&cron).map_err(|_| ScheduleError::InvalidCron(cron))?;
        let target_datetime = now.unwrap_or(Utc::now().with_timezone(&Tokyo));
        let Some(days_after) = target_datetime.checked_add_days(days) else {
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
