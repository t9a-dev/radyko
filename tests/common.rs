#![allow(dead_code)]

#[cfg(test)]
pub mod tests_common {
    use std::io::BufReader;
    use std::io::Cursor;
    use std::sync::Arc;

    use radyko::application::config;
    use radyko::application::config::RadykoConfig;
    use radyko::application::port::RadikoClient;
    use radyko::radiko::new_radiko_client;
    use tokio::sync::OnceCell;

    /// Tokyo
    pub const TEST_AREA_ID: &str = "JP13";
    pub const TEST_STATION_ID: &str = "JOAK-FM";
    pub const TEST_EMPTY_KEYWORDS_CONFIG_PATH: &str = "tests/fixtures/empty_keywords_radyko.toml";
    pub const TEST_EMPTY_RULES_CONFIG_PATH: &str = "tests/fixtures/empty_rules_radyko.toml";

    static RADIKO_CLIENT: tokio::sync::OnceCell<Arc<dyn RadikoClient>> = OnceCell::const_new();

    pub fn load_example_config() -> anyhow::Result<RadykoConfig> {
        let cursor = Cursor::new(config::EXAMPLE_CONFIG);
        let reader = BufReader::new(cursor);

        RadykoConfig::parse(reader)
    }

    pub async fn radiko_client() -> &'static Arc<dyn RadikoClient> {
        RADIKO_CLIENT
            .get_or_init(|| async { new_radiko_client(None).await.unwrap() })
            .await
    }
}
