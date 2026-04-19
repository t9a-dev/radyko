pub mod app;
pub mod cli;
pub mod commands;
mod constants;
pub mod model;
pub mod radiko;
pub mod telemetry;

#[cfg(test)]
pub mod test_helper {
    use reqwest::Client;
    use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
    use secrecy::ExposeSecret;
    use tokio::sync::{self, OnceCell};

    use crate::{app::credential::RadikoCredential, radiko::RadikoClient};

    static CLIENT: std::sync::OnceLock<ClientWithMiddleware> = std::sync::OnceLock::new();
    static RADIKO_CLIENT: sync::OnceCell<RadikoClient> = OnceCell::const_new();
    static AREA_FREE_RADIKO_CLIENT: sync::OnceCell<RadikoClient> = OnceCell::const_new();

    pub fn reqwest_client() -> &'static ClientWithMiddleware {
        CLIENT.get_or_init(|| ClientBuilder::new(Client::new()).build())
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
