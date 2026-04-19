use reqwest_middleware::ClientWithMiddleware;
use serde_derive::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use strum_macros::{AsRefStr, Display};
use thiserror::Error;

use crate::model::program::Programs;
use crate::radiko::api::endpoint::Endpoint;
use anyhow::Result;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum SearchConditionError {
    #[error("search condition keyword is required")]
    RequireKeyword,
}

#[derive(Debug, Clone)]
pub struct RadikoSearch {
    client: ClientWithMiddleware,
}

impl RadikoSearch {
    pub fn new(client: ClientWithMiddleware) -> Self {
        Self { client }
    }
    pub async fn find_program(&self, condition: &RadikoSearchCondition) -> Result<Programs> {
        if condition.key.is_empty() {
            return Err(SearchConditionError::RequireKeyword.into());
        }

        let res = &self
            .client
            .get(Endpoint::search_endpoint())
            .query(&condition.to_query_params())
            .send()
            .await?
            .text()
            .await?;

        Ok(serde_json::from_str(res)?)
    }
}

#[derive(Debug, Clone, Copy, Display, AsRefStr, Serialize, Deserialize)]
pub enum Filter {
    #[strum(to_string = "future")]
    Live,
    #[strum(to_string = "")]
    All,
    #[strum(to_string = "past")]
    TimeFree,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadikoSearchCondition {
    /// キーワード
    pub key: Vec<String>,
    pub filter: Option<Filter>,
    pub start_day: Option<String>,
    pub end_day: Option<String>,
    pub row_limit: Option<i32>,
    pub area_id: Option<Vec<String>>,
    pub station_id: Option<Vec<String>>,
    pub cur_area_id: Option<String>,
}

impl Default for RadikoSearchCondition {
    fn default() -> Self {
        Self {
            filter: Some(Filter::Live),
            row_limit: Some(50),
            key: Default::default(),
            start_day: Default::default(),
            end_day: Default::default(),
            area_id: Default::default(),
            station_id: Default::default(),
            cur_area_id: Default::default(),
        }
    }
}
impl RadikoSearchCondition {
    pub fn new() -> Self {
        Self::default()
    }
}

impl RadikoSearchCondition {
    fn to_query_params(&self) -> Vec<(String, String)> {
        let mut params = Vec::new();

        for key in &self.key {
            params.push(("key".to_string(), key.clone()));
        }

        if let Some(station_ids) = &self.station_id {
            for station_id in station_ids {
                params.push(("station_id".to_string(), station_id.clone()));
            }
        }

        if let Some(area_ids) = &self.area_id {
            for area_id in area_ids {
                params.push(("area_id".to_string(), area_id.clone()));
            }
        }

        if let Some(cur_area_id) = &self.cur_area_id {
            params.push(("cur_area_id".to_string(), cur_area_id.clone()));
        }

        if let Some(start_day) = &self.start_day {
            params.push(("start_day".to_string(), start_day.clone()));
        }

        if let Some(end_day) = &self.end_day {
            params.push(("end_day".to_string(), end_day.clone()));
        }

        if let Some(filter) = &self.filter {
            params.push(("filter".to_string(), filter.to_string().clone()));
        }

        if let Some(row_limit) = &self.row_limit {
            params.push(("row_limit".to_string(), row_limit.to_string()));
        }

        params
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Not;

    use crate::{constants::test_constants::TEST_STATION_ID, radiko::test_helper::radiko_search};

    use super::*;

    #[tokio::test]
    #[ignore = "radiko apiに依存"]
    async fn find_program_keyword_test() -> Result<()> {
        let radiko_search = radiko_search();
        let mut search_condition = RadikoSearchCondition::new();
        search_condition
            .key
            .push("オールナイトニッポン".to_string());

        let search_result = radiko_search.find_program(&search_condition).await?;
        assert!(search_result.data.is_empty().not());

        Ok(())
    }

    #[tokio::test]
    #[ignore = "radiko apiに依存"]
    async fn find_program_keyword_required_test() -> Result<()> {
        let radiko_search = radiko_search();
        let mut search_condition = RadikoSearchCondition::new();
        search_condition.station_id = Some(vec![TEST_STATION_ID.to_string()]);

        let search_result = radiko_search.find_program(&search_condition).await;
        assert_eq!(
            search_result
                .unwrap_err()
                .downcast_ref::<SearchConditionError>()
                .unwrap(),
            &SearchConditionError::RequireKeyword
        );

        Ok(())
    }
}
