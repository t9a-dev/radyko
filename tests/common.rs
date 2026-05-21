#![allow(dead_code)]

#[cfg(test)]
pub mod tests_common {
    use std::io::BufReader;
    use std::io::Cursor;

    use radyko::app::config;
    use radyko::{app::config::RadykoConfig, radiko::RadikoClient};
    use tokio::sync::OnceCell;

    /// Tokyo
    pub const TEST_AREA_ID: &str = "JP13";
    pub const TEST_STATION_ID: &str = "JOAK-FM";
    pub const TEST_EMPTY_KEYWORDS_CONFIG_PATH: &str = "tests/fixtures/empty_keywords_radyko.toml";
    pub const TEST_EMPTY_RULES_CONFIG_PATH: &str = "tests/fixtures/empty_rules_radyko.toml";

    static RADIKO_CLIENT: tokio::sync::OnceCell<RadikoClient> = OnceCell::const_new();

    pub fn load_example_config() -> anyhow::Result<RadykoConfig> {
        let cursor = Cursor::new(config::EXAMPLE_CONFIG);
        let reader = BufReader::new(cursor);

        RadykoConfig::parse(reader)
    }

    pub async fn radiko_client() -> &'static RadikoClient {
        RADIKO_CLIENT
            .get_or_init(|| async { RadikoClient::new(None).await.unwrap() })
            .await
    }
}
