use std::sync::Arc;
use tokio::sync::RwLock;

use futures::{StreamExt, stream::BoxStream};
use jiff::{ToSpan, Zoned};
use reqwest::Client;

use crate::{
    application::{self, credential::RadikoCredential, utils::Utils},
    domain::program::{Program, ProgramId, Programs, SeekStartAt, StartAt, StationId},
    radiko::api::{
        auth::RadikoAuth,
        program::RadikoProgram,
        search::{Filter, RadikoSearch, RadikoSearchCondition},
        stream::RadikoStream,
    },
};

#[derive(Debug, Clone)]
struct RadikoClient {
    auth_state: Arc<RwLock<RadikoAuth>>,
    inner: Arc<RadikoClientRef>,
}

#[derive(Debug, Clone)]
struct RadikoClientRef {
    stream: RadikoStream,
    program: RadikoProgram,
    search: RadikoSearch,
}

// RadikoClientのnewメソッドのみを公開したいので、ファクトリメソッド経由で公開
pub async fn new_radiko_client(
    credential: Option<RadikoCredential>,
) -> anyhow::Result<Arc<dyn crate::application::ports::RadikoClient>> {
    Ok(Arc::new(RadikoClient::new(credential).await?))
}

#[async_trait::async_trait]
impl application::ports::RadikoClient for RadikoClient {
    async fn refresh_auth(&self) -> anyhow::Result<()> {
        let refreshed_auth = self.auth_state.read().await.refresh_auth().await?;
        {
            let mut auth_state_guard = self.auth_state.write().await;
            *auth_state_guard = refreshed_auth;
        }

        Ok(())
    }

    async fn auth_token(&self) -> String {
        self.auth_state.read().await.auth_token().to_string()
    }

    async fn stream_url(&self, station_id: &str) -> String {
        self.inner.stream.live_stream_url(station_id).await
    }

    async fn media_list_url_for_live(&self, station_id: StationId) -> anyhow::Result<String> {
        Ok(self
            .inner
            .stream
            .get_medialist_url_for_live(&station_id.get())
            .await?
            .to_string())
    }

    async fn media_list_url_for_timefree(
        &self,
        program_id: ProgramId,
        seek_start_at: SeekStartAt,
    ) -> anyhow::Result<String> {
        self.inner
            .stream
            .get_medialist_url_for_timefree(program_id, seek_start_at)
            .await
    }

    async fn now_on_air_programs(&self, area_id: Option<&str>) -> anyhow::Result<Vec<Program>> {
        let area_id = match area_id {
            Some(area_id) => area_id.to_string(),
            None => self.auth_state.read().await.area_id().to_string(),
        };

        Ok(self
            .inner
            .program
            .now_on_air_programs(&area_id)
            .await?
            .to_vec()
            .into_iter()
            .collect::<Vec<_>>())
    }

    async fn search_programs(
        &self,
        keyword: String,
        station_id: Option<&str>,
    ) -> anyhow::Result<Programs> {
        let mut condition = RadikoSearchCondition::new();
        condition.key.push(keyword);
        if let Some(station_id) = station_id {
            condition.station_id = Some(vec![station_id.to_string()]);
        };

        self.inner.search.search_programs(&condition).await
    }

    async fn search_timefree_programs_with_keyword(
        &self,
        keyword: String,
        station_id: Option<&str>,
        start_day: Option<Zoned>,
    ) -> anyhow::Result<Programs> {
        let mut condition = RadikoSearchCondition::new();
        let _ = condition.filter.insert(Filter::Timefree);
        condition.key.push(keyword);
        if let Some(station_id) = station_id {
            condition.station_id = Some(vec![station_id.to_string()]);
        };

        let start_day_format = "%Y-%m-%d";
        let start_day = start_day.unwrap_or(
            // 指定が無い時はタイムフリーの制約である1週間前を設定する
            Utils::now_in_tz_tokyo().checked_sub(7.days())?,
        );
        let _ = condition
            .start_day
            .insert(start_day.strftime(start_day_format).to_string());

        self.inner.search.search_programs(&condition).await
    }

    async fn weekly_programs(&self, station_id: &str) -> anyhow::Result<Programs> {
        self.inner
            .program
            .weekly_programs_by_station(station_id)
            .await
    }

    async fn find_program(
        &self,
        start_at: &StartAt,
        station_id: &StationId,
    ) -> anyhow::Result<Option<Program>> {
        self.inner.program.find_program(station_id, start_at).await
    }

    fn stream_timefree_medialist_urls(
        &self,
        program_id: ProgramId,
    ) -> BoxStream<'static, anyhow::Result<String>> {
        self.inner
            .stream
            .clone()
            .stream_timefree_medialist_urls(program_id)
            .boxed()
    }
}

impl RadikoClient {
    async fn new(credential: Option<RadikoCredential>) -> anyhow::Result<Self> {
        Self::init(credential).await
    }

    async fn init(credential: Option<RadikoCredential>) -> anyhow::Result<Self> {
        let shared_radiko_auth_state =
            Arc::new(tokio::sync::RwLock::new(RadikoAuth::new(credential).await?));
        let inner = Self::init_inner(Arc::clone(&shared_radiko_auth_state)).await?;

        Ok(Self {
            auth_state: Arc::clone(&shared_radiko_auth_state),
            inner: Arc::new(inner),
        })
    }

    async fn init_inner(
        radiko_auth_state: Arc<RwLock<RadikoAuth>>,
    ) -> anyhow::Result<RadikoClientRef> {
        Ok(Self::build_inner(radiko_auth_state))
    }

    fn build_inner(radiko_auth_state: Arc<RwLock<RadikoAuth>>) -> RadikoClientRef {
        let client = Client::new();

        RadikoClientRef {
            stream: RadikoStream::new(radiko_auth_state),
            program: RadikoProgram::new(client.clone()),
            search: RadikoSearch::new(client.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test_helper::radiko_client;

    #[tokio::test]
    #[ignore = "radiko apiに依存"]
    async fn refresh_auth_test() -> anyhow::Result<()> {
        let radiko_client = radiko_client().await;
        let previous_auth_token = radiko_client.auth_token().await.to_string();

        radiko_client.refresh_auth().await?;
        let refreshed_auth_token = radiko_client.auth_token().await.to_string();

        assert_ne!(previous_auth_token, refreshed_auth_token);

        Ok(())
    }

    #[tokio::test]
    #[ignore = "radiko apiに依存"]
    async fn find_program_test() -> anyhow::Result<()> {
        let radiko_client = radiko_client().await;
        let timefree_programs = radiko_client
            .search_timefree_programs_with_keyword(
                "オールナイトニッポン".to_string(),
                Some("LFR"),
                None,
            )
            .await?
            .to_vec();
        let first_program = timefree_programs.first().unwrap();
        // 適当に選んだ番組情報から同じ番組情報を見つけられれば良い
        let program = radiko_client
            .find_program(&first_program.start_at(), &first_program.station_id())
            .await?;

        assert!(program.is_some());
        let program = program.unwrap();

        assert!(first_program.eq(&program));

        Ok(())
    }
}
