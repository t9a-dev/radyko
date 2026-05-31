use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use futures::{StreamExt, stream::BoxStream};
use tracing::{error, info};

use crate::{
    RADYKO_CONCURRENCY,
    application::{
        hls::{ByteSize, StreamHandler},
        port::RadikoClient,
        state::RecorderState,
    },
    domain::program::{Program, ProgramId, Programs},
};

#[derive(Clone)]
pub struct DownloadTimeFreeUseCase {
    http_client: reqwest::Client,
    radiko_client: Arc<dyn RadikoClient>,
    recorder_state: Arc<RecorderState>,
}

impl DownloadTimeFreeUseCase {
    pub fn new(
        http_client: reqwest::Client,
        radiko_client: Arc<dyn RadikoClient>,
        recorder_state: Arc<RecorderState>,
    ) -> Self {
        Self {
            http_client,
            radiko_client,
            recorder_state,
        }
    }

    pub async fn exec(&self, output_root_dir: &Path) -> anyhow::Result<()> {
        let program_ids = self.recorder_state.collect_aired_program_ids(None)?;
        let timefree_programs =
            Programs::resolve_program_ids(Arc::clone(&self.radiko_client), program_ids).await?;
        if timefree_programs.is_empty() {
            info!("timefree programs empty");
            return Ok(());
        }

        let mut stream_download_timefree_programs = self
            .concurrent_timefree_downloads(output_root_dir, timefree_programs, RADYKO_CONCURRENCY)
            .await;

        while let Some(result) = stream_download_timefree_programs.next().await {
            match result {
                Ok(program_id) => self.recorder_state.remove_reserved_program(program_id)?,
                Err(e) => error!("failed download timefree program error: {e:#?}"),
            }
        }

        Ok(())
    }

    async fn concurrent_timefree_downloads<'a>(
        &'a self,
        output_root_dir: &'a Path,
        programs: Vec<Program>,
        download_concurrency: usize,
    ) -> BoxStream<'a, anyhow::Result<ProgramId>> {
        futures::stream::iter(programs)
            .map(move |program| {
                let this = self.clone();
                let output_root_dir = output_root_dir.to_path_buf();
                let shared_radiko_client = Arc::clone(&this.radiko_client);
                async move {
                    info!("start download timefree {}", program.info());
                    shared_radiko_client.refresh_auth().await?;

                    let program_id = this
                        .download_timefree_program(output_root_dir, program, download_concurrency)
                        .await?;

                    Ok(program_id)
                }
            })
            .buffer_unordered(download_concurrency)
            .boxed()
    }

    async fn download_timefree_program(
        &self,
        output_root_dir: PathBuf,
        program: Program,
        download_concurrency: usize,
    ) -> anyhow::Result<ProgramId> {
        let stream_media_list_urls = self
            .concurrent_timefree_medialist_urls(&program, download_concurrency)
            .await;
        let recorded_file_path = StreamHandler::new(self.http_client.clone())
            .download_timefree_program(
                stream_media_list_urls,
                program.output_dir(output_root_dir),
                &program.output_filename(),
            )
            .await?;
        let recorded_file = fs::File::open(recorded_file_path)?;
        StreamHandler::verify_recorded_file(
            ByteSize::from_bytes(recorded_file.metadata()?.len()),
            program.on_air_duration_for_timefree(),
        )?;

        Ok(program.program_id())
    }

    async fn concurrent_timefree_medialist_urls(
        &self,
        program: &Program,
        download_concurrency: usize,
    ) -> BoxStream<'static, anyhow::Result<String>> {
        self.radiko_client
            .concurrent_timefree_medialist_urls(program.program_id(), download_concurrency)
    }
}
