pub mod app;
pub mod cli;
pub mod commands;
mod constants;
pub mod model;
pub mod radiko;
pub mod telemetry;

pub const RADYKO_TZ_NAME: &str = "Asia/Tokyo";
/// 非同期処理を並行処理する際の同時数
pub const RADYKO_CONCURRENCY: usize = 2;
#[cfg(test)]
pub mod test_helper {

    use std::io::{BufReader, Cursor};

    use anyhow::Context;
    use jiff::{Zoned, civil::DateTime};
    use reqwest::Client;
    use tokio::sync::{self, OnceCell};

    use crate::{
        RADYKO_TZ_NAME,
        app::config::{self, RadykoConfig},
        radiko::{RadikoClient, credential::RadikoCredential},
    };

    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    static RADIKO_CLIENT: sync::OnceCell<RadikoClient> = OnceCell::const_new();
    static AREA_FREE_RADIKO_CLIENT: sync::OnceCell<RadikoClient> = OnceCell::const_new();

    pub fn reqwest_client() -> &'static reqwest::Client {
        CLIENT.get_or_init(Client::new)
    }

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

    pub async fn area_free_radiko_client() -> &'static RadikoClient {
        AREA_FREE_RADIKO_CLIENT
            .get_or_init(|| async {
                let credential = RadikoCredential::load_from_env_file().unwrap();
                RadikoClient::new(Some(credential)).await.unwrap()
            })
            .await
    }

    /// "%Y-%m-%d %H:%M:%S" -> Zoned in Asia/Tokyo
    pub fn parse_datetime_in_tz_tokyo(s: &str) -> Zoned {
        DateTime::strptime("%Y-%m-%d %H:%M:%S", s)
            .with_context(|| format!("parse_datetime_in_tz_tokyo error s: {s}"))
            .unwrap()
            .in_tz(RADYKO_TZ_NAME)
            .unwrap()
    }
}
