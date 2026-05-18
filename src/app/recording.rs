use std::{path::PathBuf, sync::Arc, time::Duration};

use tracing::{info, trace};

use crate::{app::hls::StreamHandler, model::program::program::Program, radiko::RadikoClient};

pub struct RetryWithBackOffPolicy {
    pub max_attempts: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
}

pub async fn start_for_live(
    radiko_client: &RadikoClient,
    program: Arc<Program>,
    output_root_dir: PathBuf,
) -> anyhow::Result<()> {
    let program_info = program.info();
    let on_air_duration = program.on_air_duration();
    trace!(
        "recording for live: on_air_duration_secs: {:#?}, program: {}",
        on_air_duration, program_info
    );

    let media_list_url = program.media_list_url_for_live(radiko_client).await?;
    let stream_handler = StreamHandler::new(reqwest::Client::new());
    info!("start recording for live: {}", program_info);
    stream_handler
        .start_recording(
            media_list_url,
            program.output_dir(output_root_dir),
            &program.output_filename(),
            Duration::from_secs(on_air_duration.0),
        )
        .await
}
