use std::sync::Arc;

use crate::model::{Program, program::Programs};
use crate::radiko::xml::program::RadikoProgramXml;
use anyhow::{Context, Result};
use jiff::Zoned;

use crate::radiko::api::endpoint::Endpoint;

#[derive(Debug, Clone)]
pub struct RadikoProgram {
    inner: Arc<RadikoProgramRef>,
}

#[derive(Debug)]
struct RadikoProgramRef {
    client: reqwest::Client,
}

impl RadikoProgram {
    pub fn new(client: reqwest::Client) -> Self {
        Self {
            inner: Arc::new(RadikoProgramRef { client }),
        }
    }

    pub async fn now_on_air_programs(&self, area_id: &str) -> Result<Programs> {
        let res = self
            .inner
            .client
            .get(Endpoint::now_on_air_programs(area_id))
            .send()
            .await?
            .text()
            .await?;

        let radiko_program: RadikoProgramXml = quick_xml::de::from_str(&res)?;

        Ok(Programs::from(radiko_program))
    }

    pub async fn weekly_programs_by_station(&self, station_id: &str) -> Result<Programs> {
        let endpoint = Endpoint::weekly_programs_endpoint(station_id);
        let res = self
            .inner
            .client
            .get(&endpoint)
            .send()
            .await?
            .text()
            .await?;

        let radiko_program: RadikoProgramXml = quick_xml::de::from_str(&res)
            .with_context(|| format!("failed deserialize programs. endpoint: {endpoint}"))?;

        Ok(Programs::from(radiko_program))
    }

    pub async fn find_program(&self, station_id: &str, start_at: Zoned) -> Result<Option<Program>> {
        let endpoint = Endpoint::weekly_programs_endpoint(station_id);
        let res = self
            .inner
            .client
            .get(&endpoint)
            .send()
            .await?
            .text()
            .await?;

        let radiko_program: RadikoProgramXml = quick_xml::de::from_str(&res)
            .with_context(|| format!("failed deserialize programs. endpoint: {endpoint}"))?;
        let programs = Programs::from(radiko_program);

        Ok(programs.find_program(start_at))
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Not;

    use crate::{constants::test_constants::TEST_STATION_ID, radiko::test_helper::radiko_program};

    use super::*;

    #[tokio::test]
    #[ignore = "radiko apiに依存"]
    async fn get_now_on_air_programs_smoke() -> Result<()> {
        let radiko_program = radiko_program();
        let programs = radiko_program.now_on_air_programs("JP13").await?;
        assert!(programs.data.is_empty().not());

        Ok(())
    }

    #[tokio::test]
    #[ignore = "radiko apiに依存"]
    async fn weekly_programs_from_station_smoke() -> Result<()> {
        let radiko_program = radiko_program();
        let station_weekly_programs = radiko_program
            .weekly_programs_by_station(TEST_STATION_ID)
            .await?;
        assert!(station_weekly_programs.data.is_empty().not());

        Ok(())
    }
}
