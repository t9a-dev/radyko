#![allow(dead_code)]

#[cfg(test)]
pub mod tests_common {
    use std::io::BufReader;
    use std::io::Cursor;

    use chrono::DateTime;
    use chrono_tz::Tz;

    use radyko::app::config;
    use radyko::model::Program;
    use radyko::{app::config::RadykoConfig, radiko::RadikoClient};
    use tokio::sync::OnceCell;

    /// Tokyo
    pub const TEST_AREA_ID: &str = "JP13";
    pub const TEST_STATION_ID: &str = "JOAK-FM";
    pub const TEST_EMPTY_KEYWORDS_CONFIG_PATH: &str = "tests/fixtures/empty_keywords_config.toml";
    pub const TEST_EMPTY_RULES_CONFIG_PATH: &str = "tests/fixtures/empty_rules_config.toml";

    static RADIKO_CLIENT: tokio::sync::OnceCell<RadikoClient> = OnceCell::const_new();

    pub fn load_example_config() -> anyhow::Result<RadykoConfig> {
        let cursor = Cursor::new(config::EXAMPLE_CONFIG);
        let reader = BufReader::new(cursor);

        Ok(RadykoConfig::parse(reader)?)
    }

    pub async fn radiko_client() -> &'static RadikoClient {
        RADIKO_CLIENT
            .get_or_init(|| async { RadikoClient::new().await.unwrap() })
            .await
    }

    pub trait TestProgram {
        fn with_start_end_time(&self, start: DateTime<Tz>, end: DateTime<Tz>) -> Self;
    }

    impl TestProgram for Program {
        /// テスト用に放送時間を差し替えたProgramを返す
        fn with_start_end_time(&self, start: DateTime<Tz>, end: DateTime<Tz>) -> Self {
            Self {
                start_time: start,
                end_time: end,
                ..self.clone()
            }
        }
    }
}
