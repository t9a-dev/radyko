use std::{borrow::Cow, collections::HashMap, str::FromStr, sync::Arc};

use anyhow::{Result, anyhow};

use base64::{Engine, engine::general_purpose};
use regex::Regex;
use reqwest::{
    Client, Url,
    cookie::{self, Jar},
    header::HeaderMap,
};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::radiko::api::endpoint::Endpoint;

#[derive(Debug, Clone)]
pub struct RadikoAuthedClient(reqwest::Client);

#[derive(Debug, Clone)]
pub struct RadikoAuth {
    inner: Arc<RadikoAuthRef>,
}

#[derive(Debug)]
struct RadikoAuthRef {
    area_id: String,
    area_free: bool,
    http_client: RadikoAuthedClient,
    auth_token: String,
    stream_lsid: String,
    email_address: Option<SecretString>,
    password: Option<SecretString>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LoginResponse {
    twitter_name: Option<String>,
    status: String,
    unpaid: String,
    radiko_session: String,
    areafree: String,
    member_ukey: String,
    facebook_name: Option<String>,
    privileges: Vec<String>,
    paid_member: String,
}

impl RadikoAuth {
    pub async fn new() -> anyhow::Result<Self> {
        Self::init(None, None).await
    }

    pub async fn new_area_free(email_address: &str, password: &str) -> anyhow::Result<Self> {
        Self::init(
            Some(SecretString::new(email_address.into())),
            Some(SecretString::new(password.into())),
        )
        .await
    }

    pub fn area_id(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.inner.area_id)
    }

    pub fn area_free(&self) -> bool {
        self.inner.area_free
    }

    pub fn http_client(&self) -> Client {
        self.inner.http_client.0.clone()
    }

    pub fn auth_token(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.inner.auth_token)
    }

    pub fn lsid(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.inner.stream_lsid)
    }

    pub async fn refresh_auth(&self) -> Result<Self> {
        Self::init(
            self.inner.email_address.clone(),
            self.inner.password.clone(),
        )
        .await
    }

    async fn init(mail: Option<SecretString>, pass: Option<SecretString>) -> Result<Self> {
        let auth1_url = Endpoint::auth1_endpoint();
        let auth2_url = Endpoint::auth2_endpoint();
        let auth_key = Self::get_public_auth_key().await?;

        // get area_id
        let response_body = Client::new()
            .get(Endpoint::area_id_endpoint())
            .send()
            .await?
            .text()
            .await?;

        let area_id_pattern = Regex::new(r"[A-Z]{2}[0-9]{2}")?;
        let Some(area_id_caps) = area_id_pattern.captures(&response_body) else {
            panic!("failed get area_id. not found pattern area_id");
        };
        let default_area_id = area_id_caps[0].to_string();

        // login
        let (is_area_free, cookie) = match (mail.clone(), pass.clone()) {
            (Some(mail), Some(pass)) => (
                true,
                RadikoAuth::login(mail.clone().expose_secret(), pass.clone().expose_secret())
                    .await?,
            ),
            _ => (false, Arc::new(Jar::default())),
        };
        let logined_client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .cookie_provider(cookie.clone())
            .build()?;

        // auth1
        let mut headers = HeaderMap::new();
        headers.insert("X-Radiko-App", "pc_html5".parse()?);
        headers.insert("X-Radiko-App-Version", "0.0.1".parse()?);
        headers.insert("X-Radiko-User", "dummy_user".parse()?);
        headers.insert("X-Radiko-Device", "pc".parse()?);

        let res_auth1 = logined_client
            .get(auth1_url)
            .headers(headers)
            .send()
            .await?;

        // auth2
        let auth_token = res_auth1
            .headers()
            .get("X-Radiko-Authtoken")
            .unwrap()
            .to_str()?;
        let offset = res_auth1
            .headers()
            .get("X-Radiko-KeyOffset")
            .unwrap()
            .to_str()?
            .parse::<usize>()?;
        let length = res_auth1
            .headers()
            .get("X-Radiko-KeyLength")
            .unwrap()
            .to_str()?
            .parse::<usize>()?;
        let partial_key = general_purpose::STANDARD.encode(&auth_key[offset..offset + length]);

        let mut headers = HeaderMap::new();
        headers.insert("X-Radiko-Authtoken", auth_token.parse()?);
        headers.insert("X-Radiko-Partialkey", partial_key.parse()?);
        headers.insert("X-Radiko-User", "dummy_user".parse()?);
        headers.insert("X-Radiko-Device", "pc".parse()?);

        let res_auth2 = logined_client
            .get(&auth2_url)
            .headers(headers.clone())
            .send()
            .await?;
        if !res_auth2.status().is_success() {
            return Err(anyhow!("error auth2 request: {}", res_auth2.text().await?));
        }

        let authed_client = Client::builder()
            .cookie_provider(cookie)
            .default_headers(headers)
            .build()?;

        // cookieに設定されるa_expはmd5ハッシュ現在日時から適当に生成しているだけ
        // https://radiko.jp/apps/js/common.js?_=20250306
        let lsid = super::utils::Utils::generate_md5_hash();

        info!("area id: {}", default_area_id);
        info!("area free: {}", is_area_free);

        Ok(Self {
            inner: Arc::new(RadikoAuthRef {
                area_id: default_area_id.to_string(),
                area_free: is_area_free,
                http_client: RadikoAuthedClient(authed_client),
                auth_token: auth_token.to_string(),
                stream_lsid: lsid,
                email_address: mail,
                password: pass,
            }),
        })
    }

    async fn get_public_auth_key() -> anyhow::Result<String> {
        // https://github.com/miyagawa/ripdiko/blob/e9080f99c4c45b112256d822802f3dd56ab908f1/bin/ripdiko#L66
        let url = "https://radiko.jp/apps/js/playerCommon.js";
        let response_body = reqwest::get(url).await?.text().await?;
        let auth_key_pattern =
            regex::Regex::new(r"new RadikoJSPlayer\(.*?,.*?,.'(?P<auth_key>\w+)'")?;
        let Some(auth_key_caps) = auth_key_pattern.captures(&response_body) else {
            // public key from https://radiko.jp/apps/js/playerCommon.js
            return Ok("bcd151073c03b352e1ef2fd66c32209da9ca0afa".to_string());
        };

        Ok(auth_key_caps["auth_key"].to_string())
    }

    async fn login(mail: &str, pass: &str) -> Result<Arc<cookie::Jar>> {
        let mut login_info = HashMap::new();
        login_info.insert("mail", mail);
        login_info.insert("pass", pass);
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
            return Err(anyhow!(
                "login check failed: {}",
                login_check_res.text().await?
            ));
        }

        Ok(jar)
    }
}

#[cfg(test)]
mod tests {
    use crate::radiko::test_helper::{AuthType, radiko_auth};

    use super::*;

    #[tokio::test]
    #[ignore = "エリアフリー会員情報を持つことに依存しているテスト"]
    async fn init_area_free_client_smoke() -> Result<()> {
        let auth_manager = radiko_auth(AuthType::AreaFree).await;
        assert!(auth_manager.area_free());

        Ok(())
    }

    #[tokio::test]
    #[ignore = "radiko apiに依存"]
    async fn init_not_area_free_client_smoke() -> Result<()> {
        let auth_manager = radiko_auth(AuthType::Normal).await;
        assert!(!auth_manager.area_free());

        Ok(())
    }

    #[tokio::test]
    #[ignore = "radiko apiに依存"]
    async fn refresh_auth_test() -> Result<()> {
        let auth_manager = radiko_auth(AuthType::Normal).await;
        let refreshed_auth_manager = auth_manager.refresh_auth().await?;

        assert_ne!(
            auth_manager.auth_token(),
            refreshed_auth_manager.auth_token()
        );

        Ok(())
    }
}
