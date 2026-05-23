use std::{collections::HashMap, env, path::PathBuf, str::FromStr, sync::Arc};

use anyhow::bail;
use reqwest::{
    Client, Url,
    cookie::{self, Jar},
};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use tracing::{info, warn};

use crate::radiko::api::Endpoint;

#[derive(Debug, Clone)]
pub struct RadikoCredential {
    email_address: SecretString,
    password: SecretString,
}

impl RadikoCredential {
    pub fn load_from_env_file() -> Option<RadikoCredential> {
        let env_file_path = PathBuf::from(".env");
        let _ = dotenvy::from_path(&env_file_path);
        let mail = env::var("RADIKO_AREA_FREE_MAIL");
        let password = env::var("RADIKO_AREA_FREE_PASSWORD");
        match (mail, password) {
            (Ok(mail), Ok(password)) => {
                info!("success load radiko credential from environment");
                Some(RadikoCredential {
                    email_address: SecretString::new(mail.into()),
                    password: SecretString::new(password.into()),
                })
            }
            _ => {
                warn!(
                    "failed load radiko credential from environment env_file_path: {env_file_path:#?}"
                );
                None
            }
        }
    }

    pub async fn login(&self) -> anyhow::Result<Arc<cookie::Jar>> {
        let mut login_info = HashMap::new();
        login_info.insert("mail", self.email_address.expose_secret());
        login_info.insert("pass", self.password.expose_secret());

        let login_res: LoginResponse = Client::new()
            .post(Endpoint::login_endpoint())
            .form(&login_info)
            .send()
            .await?
            .json()
            .await?;
        let cookie = format!("radiko_session={}", login_res.radiko_session);
        let jar = Arc::new(Jar::default());
        jar.add_cookie_str(&cookie, &Url::from_str(Endpoint::RADIKO_HOST)?);

        let login_check_res = Client::builder()
            .cookie_provider(jar.clone())
            .build()?
            .get(Endpoint::LOGIN_CHECK_URL)
            .send()
            .await?;

        if !login_check_res.status().is_success() {
            bail!(format!(
                "login check failed: {}",
                login_check_res.text().await?
            ));
        }

        Ok(jar)
    }
}

#[derive(Debug, Clone, Deserialize)]
struct LoginResponse {
    radiko_session: String,
    // twitter_name: Option<String>,
    // status: String,
    // unpaid: String,
    // areafree: String,
    // member_ukey: String,
    // facebook_name: Option<String>,
    // privileges: Vec<String>,
    // paid_member: String,
}
