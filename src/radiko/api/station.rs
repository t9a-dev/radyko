use std::sync::Arc;

use anyhow::Result;
use reqwest::Client;

use crate::{
    model::{
        region::{Region, RegionStations},
        station::Stations,
    },
    radiko::{
        api::endpoint::Endpoint,
        xml::{region::RegionXml, station::StationsXml},
    },
};

#[derive(Debug, Clone)]
pub struct RadikoStation {
    inner: Arc<RadikoStationRef>,
}

#[derive(Debug)]
struct RadikoStationRef {
    client: Client,
}

impl RadikoStation {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RadikoStationRef {
                client: Client::new(),
            }),
        }
    }

    pub async fn stations_from_area_id(&self, area_id: &str) -> Result<Stations> {
        let res = self
            .inner
            .client
            .get(Endpoint::station_list_from_area_id_endpoint(area_id))
            .send()
            .await?
            .text()
            .await?;

        let radiko_station: StationsXml = quick_xml::de::from_str(&res)?;

        Ok(Stations::from(radiko_station))
    }

    pub async fn stations_all(&self) -> Result<Vec<RegionStations>> {
        let res = self
            .inner
            .client
            .get(Endpoint::station_list_all_endpoint())
            .send()
            .await?
            .text()
            .await?;

        let region: RegionXml = quick_xml::de::from_str(&res)?;

        Ok(Region::from(region).stations_groups)
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Not;

    use crate::radiko::test_helper::radiko_station;

    use super::*;

    #[tokio::test]
    #[ignore = "radiko apiに依存"]
    async fn stations_from_area_id_smoke() -> Result<()> {
        let radiko_station = radiko_station();
        let area_id = "JP13"; // TOKYO JAPAN
        let stations_from_area = radiko_station.stations_from_area_id(area_id).await?;
        assert_eq!(stations_from_area.area_id, area_id);
        assert_eq!(stations_from_area.area_name, "TOKYO JAPAN");
        assert!(stations_from_area.data.is_empty().not());

        Ok(())
    }

    #[tokio::test]
    #[ignore = "radiko apiに依存"]
    async fn stations_all_smoke() -> Result<()> {
        let radiko_station = radiko_station();
        let all_stations = radiko_station.stations_all().await?;
        assert!(all_stations.is_empty().not());

        Ok(())
    }
}
