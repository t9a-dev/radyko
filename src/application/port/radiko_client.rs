use futures::stream::BoxStream;
use jiff::Zoned;

use crate::domain::program::{Program, ProgramId, Programs, SeekStartAt, StartAt, StationId};

#[async_trait::async_trait]
pub trait RadikoClient: Send + Sync {
    async fn refresh_auth(&self) -> anyhow::Result<()>;

    async fn auth_token(&self) -> String;

    async fn stream_url(&self, station_id: &str) -> String;

    async fn media_list_url_for_live(&self, station_id: StationId) -> anyhow::Result<String>;

    async fn media_list_url_for_timefree(
        &self,
        program_id: ProgramId,
        seek_start_at: SeekStartAt,
    ) -> anyhow::Result<String>;

    async fn now_on_air_programs(&self, area_id: Option<&str>) -> anyhow::Result<Vec<Program>>;

    async fn search_programs(
        &self,
        keyword: String,
        station_id: Option<&str>,
    ) -> anyhow::Result<Programs>;

    async fn search_timefree_programs_with_keyword(
        &self,
        keyword: String,
        station_id: Option<&str>,
        start_day: Option<Zoned>,
    ) -> anyhow::Result<Programs>;

    async fn weekly_programs(&self, station_id: &str) -> anyhow::Result<Programs>;

    async fn find_program(
        &self,
        start_at: &StartAt,
        station_id: &StationId,
    ) -> anyhow::Result<Option<Program>>;

    fn concurrent_timefree_medialist_urls(
        &self,
        program_id: ProgramId,
        download_concurrency: usize,
    ) -> BoxStream<'static, anyhow::Result<String>>;
}
