use std::sync::Arc;

use http_cache_reqwest::{CACacheManager, Cache, CacheMode, HttpCache, HttpCacheOptions};
use reqwest::Client;
use reqwest_middleware::ClientBuilder;
use tempfile::TempDir;

use crate::{
    model::program::{Program, Programs},
    radiko::api::{
        auth::RadikoAuth,
        program::RadikoProgram,
        search::{RadikoSearch, RadikoSearchCondition},
        stream::RadikoStream,
    },
};

#[derive(Debug, Clone)]
pub struct RadikoClient {
    default_area_id: Arc<String>,
    inner: Arc<RadikoClientRef>,
}

#[derive(Debug, Clone)]
struct RadikoClientRef {
    auth: RadikoAuth,
    stream: RadikoStream,
    program: RadikoProgram,
    search: RadikoSearch,
    http_cache_dir: Arc<TempDir>,
}

impl RadikoClient {
    pub async fn new_area_free(
        email_address: &str,
        password: &str,
        http_cache_dir: Arc<TempDir>,
    ) -> anyhow::Result<RadikoClient> {
        Self::init(Some(email_address), Some(password), http_cache_dir).await
    }

    pub async fn new(http_cache_dir: Arc<TempDir>) -> anyhow::Result<RadikoClient> {
        Self::init(None, None, http_cache_dir).await
    }

    pub async fn refresh_auth(&self) -> anyhow::Result<RadikoClient> {
        let refreshed_auth = self.inner.auth.refresh_auth().await?;
        let inner = Self::build_inner(refreshed_auth, Arc::clone(&self.inner.http_cache_dir));

        Ok(Self {
            default_area_id: Arc::new(inner.auth.area_id().to_string()),
            inner: Arc::new(inner),
        })
    }

    pub async fn auth_token(&self) -> String {
        self.inner.auth.auth_token().to_string()
    }

    pub async fn stream_url(&self, station_id: &str) -> String {
        self.inner.stream.stream_url(station_id)
    }

    pub async fn media_list_url(&self, station_id: &str) -> anyhow::Result<String> {
        Ok(self
            .inner
            .stream
            .get_medialist_url(station_id)
            .await?
            .to_string())
    }

    pub async fn now_on_air_programs(&self, area_id: Option<&str>) -> anyhow::Result<Vec<Program>> {
        let area_id = area_id.unwrap_or(&self.default_area_id);
        Ok(self
            .inner
            .program
            .now_on_air_programs(area_id)
            .await?
            .data
            .into_iter()
            .collect::<Vec<_>>())
    }

    pub async fn search_programs(
        &self,
        keyword: String,
        station_id: Option<&str>,
    ) -> anyhow::Result<Programs> {
        let mut condition = RadikoSearchCondition::new();
        condition.key.push(keyword);
        if let Some(station_id) = station_id {
            condition.station_id = Some(vec![station_id.to_string()]);
        };

        self.inner.search.find_program(&condition).await
    }

    pub async fn weekly_programs(&self, station_id: &str) -> anyhow::Result<Programs> {
        self.inner
            .program
            .weekly_programs_from_station(station_id)
            .await
    }

    async fn init(
        email_address: Option<&str>,
        password: Option<&str>,
        http_cache_dir: Arc<TempDir>,
    ) -> anyhow::Result<Self> {
        let inner = Self::init_inner(email_address, password, http_cache_dir).await?;

        Ok(Self {
            default_area_id: Arc::new(inner.auth.area_id().to_string()),
            inner: Arc::new(inner),
        })
    }

    async fn init_inner(
        email_address: Option<&str>,
        password: Option<&str>,
        http_cache_dir: Arc<TempDir>,
    ) -> anyhow::Result<RadikoClientRef> {
        let is_area_free = email_address.is_some() && password.is_some();
        let radiko_auth = if is_area_free {
            RadikoAuth::new_area_free(email_address.unwrap(), password.unwrap()).await
        } else {
            RadikoAuth::new().await
        };

        Ok(Self::build_inner(radiko_auth, http_cache_dir))
    }

    fn build_inner(radiko_auth: RadikoAuth, http_cache_dir: Arc<TempDir>) -> RadikoClientRef {
        let client = ClientBuilder::new(Client::new())
            .with(Cache(HttpCache {
                mode: CacheMode::Default,
                manager: CACacheManager::new(http_cache_dir.path().into(), false),
                options: HttpCacheOptions::default(),
            }))
            .build();

        RadikoClientRef {
            auth: radiko_auth.clone(),
            stream: RadikoStream::new(radiko_auth.clone()),
            program: RadikoProgram::new(client.clone()),
            search: RadikoSearch::new(client.clone()),
            http_cache_dir,
        }
    }
}
