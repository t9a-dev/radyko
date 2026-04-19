use crate::{
    app::types::{Keyword, Station},
    model::program::Programs,
    radiko::RadikoClient,
};

pub async fn search_programs(
    radiko_client: &RadikoClient,
    keyword: Keyword,
    station: Station,
) -> anyhow::Result<Programs> {
    match station {
        Station::Id(station_id) => {
            radiko_client
                .search_programs(keyword.0, Some(&station_id))
                .await
        }
        Station::Nationwide => radiko_client.search_programs(keyword.0, None).await,
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Not;

    use super::*;
    use crate::{constants::test_constants::TEST_STATION_ID, test_helper::radiko_client};

    #[tokio::test]
    #[ignore = "radiko apiに依存"]
    async fn search_programs_smoke() -> anyhow::Result<()> {
        let radiko_client = radiko_client().await;

        assert!(
            search_programs(
                &radiko_client,
                Keyword("オールナイトニッポン".to_string()),
                Station::Nationwide
            )
            .await?
            .data
            .is_empty()
            .not()
        );

        assert!(
            search_programs(
                &radiko_client,
                Keyword("クラシック".to_string()),
                Station::Id(TEST_STATION_ID.to_string())
            )
            .await?
            .data
            .is_empty()
            .not()
        );

        Ok(())
    }
}
