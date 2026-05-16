use std::sync::Arc;

use futures::Stream;
use jiff::{ToSpan, Zoned};
use reqwest::Client;

use crate::{
    app::utils::Utils,
    model::program::{Program, Programs},
    radiko::{
        api::{
            auth::RadikoAuth,
            program::RadikoProgram,
            search::{Filter, RadikoSearch, RadikoSearchCondition},
            stream::RadikoStream,
        },
        credential::RadikoCredential,
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
}

// TODO: 対応するインターフェースを作る オブジェクト設計スタイルガイド6章.5あたり
impl RadikoClient {
    pub async fn new(credential: Option<RadikoCredential>) -> anyhow::Result<Self> {
        Self::init(credential).await
    }

    pub async fn refresh_auth(&self) -> anyhow::Result<Self> {
        let refreshed_auth = self.inner.auth.refresh_auth().await?;
        let inner = Self::build_inner(refreshed_auth);

        Ok(Self {
            default_area_id: Arc::new(inner.auth.area_id().to_string()),
            inner: Arc::new(inner),
        })
    }

    pub async fn auth_token(&self) -> String {
        self.inner.auth.auth_token().to_string()
    }

    pub async fn stream_url(&self, station_id: &str) -> String {
        self.inner.stream.live_stream_url(station_id)
    }

    pub async fn media_list_url_for_live(&self, station_id: &str) -> anyhow::Result<String> {
        Ok(self
            .inner
            .stream
            .get_medialist_url_for_live(station_id)
            .await?
            .to_string())
    }

    pub async fn media_list_url_for_timefree(
        &self,
        station_id: String,
        start_at: Zoned,
        end_at: Zoned,
        seek: Zoned,
    ) -> anyhow::Result<String> {
        self.inner
            .stream
            .get_medialist_url_for_timefree(station_id, start_at, end_at, seek)
            .await
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

    pub async fn search_timefree_programs_with_keyword(
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

        self.inner.search.find_program(&condition).await
    }

    pub async fn weekly_programs(&self, station_id: &str) -> anyhow::Result<Programs> {
        self.inner
            .program
            .weekly_programs_by_station(station_id)
            .await
    }

    pub async fn find_program(
        &self,
        start_at: Zoned,
        station_id: &str,
    ) -> anyhow::Result<Option<Program>> {
        self.inner.program.find_program(station_id, start_at).await
    }

    pub fn stream_timefree_medialist_urls(
        &self,
        station_id: String,
        start_at: Zoned,
        end_at: Zoned,
    ) -> impl Stream<Item = anyhow::Result<String>> {
        self.inner
            .stream
            .stream_timefree_medialist_urls(station_id, start_at, end_at)
    }

    async fn init(credential: Option<RadikoCredential>) -> anyhow::Result<Self> {
        let inner = Self::init_inner(credential).await?;

        Ok(Self {
            default_area_id: Arc::new(inner.auth.area_id().to_string()),
            inner: Arc::new(inner),
        })
    }

    async fn init_inner(credential: Option<RadikoCredential>) -> anyhow::Result<RadikoClientRef> {
        let radiko_auth = RadikoAuth::new(credential).await;

        Ok(Self::build_inner(radiko_auth?))
    }

    fn build_inner(radiko_auth: RadikoAuth) -> RadikoClientRef {
        let client = Client::new();

        RadikoClientRef {
            auth: radiko_auth.clone(),
            stream: RadikoStream::new(radiko_auth.clone()),
            program: RadikoProgram::new(client.clone()),
            search: RadikoSearch::new(client.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{radiko::api::endpoint::Endpoint, test_helper::radiko_client};

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
            .await?;
        let first_program = timefree_programs.data.first().unwrap();
        println!(
            "first_program_start_time: {}",
            first_program
                .start_time
                .clone()
                .strftime(Endpoint::DATETIME_FORMAT)
        );
        // 適当に選んだ番組情報から同じ番組情報を見つけられれば良い
        let program = radiko_client
            .find_program(first_program.start_time.clone(), &first_program.station_id)
            .await?;

        assert!(program.is_some());
        let program = program.unwrap();

        assert_eq!(first_program.station_id, program.station_id);
        assert_eq!(first_program.title, program.title);
        assert_eq!(first_program.start_time, program.start_time);
        assert_eq!(first_program.end_time, program.end_time);

        Ok(())
    }
}
