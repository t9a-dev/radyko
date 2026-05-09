pub mod app;
pub mod cli;
pub mod commands;
mod constants;
pub mod model;
pub mod radiko;
pub mod telemetry;

/// 非同期処理を並行処理する際の同時数
pub const RADYKO_CONCURRENCY: usize = 2;
#[cfg(test)]
pub mod test_helper {

    use std::io::{BufReader, Cursor};

    use reqwest::Client;
    use secrecy::ExposeSecret;
    use tokio::sync::{self, OnceCell};

    use crate::{
        app::{
            config::{self, RadykoConfig},
            credential::RadikoCredential,
        },
        radiko::RadikoClient,
    };

    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    static RADIKO_CLIENT: sync::OnceCell<RadikoClient> = OnceCell::const_new();
    static AREA_FREE_RADIKO_CLIENT: sync::OnceCell<RadikoClient> = OnceCell::const_new();

    pub fn reqwest_client() -> &'static reqwest::Client {
        CLIENT.get_or_init(|| Client::new())
    }

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

    pub async fn area_free_radiko_client() -> &'static RadikoClient {
        AREA_FREE_RADIKO_CLIENT
            .get_or_init(|| async {
                let credential = RadikoCredential::load_credential().unwrap();
                RadikoClient::new_area_free(
                    credential.email_address.expose_secret(),
                    credential.password.expose_secret(),
                )
                .await
                .unwrap()
            })
            .await
    }
}
