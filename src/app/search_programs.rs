use crate::{
    app::types::{Keyword, Station},
    radiko::RadikoClient,
    radiko::model::program::Programs,
};

pub async fn search_programs(
    radiko_client: &RadikoClient,
    keyword: Keyword,
    station: Station,
) -> anyhow::Result<Programs> {
    match station {
        Station::Id(station_id) => {
            radiko_client
                .search_programs(keyword.get(), Some(&station_id))
                .await
        }
        Station::Nationwide => radiko_client.search_programs(keyword.get(), None).await,
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
                radiko_client,
                Keyword::new("オールナイトニッポン".to_string()),
                Station::Nationwide
            )
            .await?
            .to_vec()
            .is_empty()
            .not()
        );

        assert!(
            search_programs(
                radiko_client,
                Keyword::new("クラシック".to_string()),
                Station::Id(TEST_STATION_ID.to_string())
            )
            .await?
            .to_vec()
            .is_empty()
            .not()
        );

        Ok(())
    }
}
