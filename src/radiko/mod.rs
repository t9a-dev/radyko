pub mod api;
mod client;
mod converter;
pub(crate) mod xml;
pub use converter::jst_datetime;

pub use client::RadikoClient;

#[cfg(test)]
mod test_helper {
    use anyhow::Context;
    use reqwest::Client;
    use secrecy::ExposeSecret;

    use crate::{
        app::credential::RadikoCredential,
        radiko::api::{
            auth::RadikoAuth, program::RadikoProgram, search::RadikoSearch, station::RadikoStation,
            stream::RadikoStream,
        },
    };

    pub enum AuthType {
        Normal,
        AreaFree,
    }

    // tokio::sync
    static RADIKO_AUTH: tokio::sync::OnceCell<RadikoAuth> = tokio::sync::OnceCell::const_new();
    static RADIKO_AUTH_AREA_FREE: tokio::sync::OnceCell<RadikoAuth> =
        tokio::sync::OnceCell::const_new();
    static RADIKO_STREAM: tokio::sync::OnceCell<RadikoStream> = tokio::sync::OnceCell::const_new();
    // std::sync
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    static RADIKO_STATION: std::sync::OnceLock<RadikoStation> = std::sync::OnceLock::new();
    static RADIKO_SEARCH: std::sync::OnceLock<RadikoSearch> = std::sync::OnceLock::new();
    static RADIKO_PROGRAM: std::sync::OnceLock<RadikoProgram> = std::sync::OnceLock::new();

    pub fn reqwest_client() -> &'static reqwest::Client {
        CLIENT.get_or_init(Client::new)
    }

    pub async fn radiko_auth(auth_type: AuthType) -> &'static RadikoAuth {
        match auth_type {
            AuthType::Normal => {
                RADIKO_AUTH
                    .get_or_init(|| async { RadikoAuth::new().await.unwrap() })
                    .await
            }
            AuthType::AreaFree => {
                RADIKO_AUTH_AREA_FREE
                    .get_or_init(|| async {
                        let credential = RadikoCredential::load_credential()
                            .context("エリアフリー会員情報の読み込みに失敗")
                            .unwrap();
                        RadikoAuth::new_area_free(
                            credential.email_address.expose_secret(),
                            credential.password.expose_secret(),
                        )
                        .await
                        .unwrap()
                    })
                    .await
            }
        }
    }

    pub async fn radiko_stream(auth_type: AuthType) -> &'static RadikoStream {
        RADIKO_STREAM
            .get_or_init(|| async { RadikoStream::new(radiko_auth(auth_type).await.clone()) })
            .await
    }

    pub fn radiko_station() -> &'static RadikoStation {
        RADIKO_STATION.get_or_init(RadikoStation::new)
    }

    pub fn radiko_search() -> &'static RadikoSearch {
        RADIKO_SEARCH.get_or_init(|| RadikoSearch::new(reqwest_client().clone()))
    }

    pub fn radiko_program() -> &'static RadikoProgram {
        RADIKO_PROGRAM.get_or_init(|| RadikoProgram::new(reqwest_client().clone()))
    }
}
