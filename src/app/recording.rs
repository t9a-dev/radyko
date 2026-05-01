use std::{sync::Arc, time::Duration};

use tracing::{info, trace};

use crate::{
    app::{hls::StreamHandler, program_reserver::ReserveProgram},
    radiko::RadikoClient,
};

pub struct RetryWithBackOffPolicy {
    pub max_attempts: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
}

pub async fn start_for_live(
    radiko_client: RadikoClient,
    program: Arc<ReserveProgram>,
) -> anyhow::Result<()> {
    recording_for_live(&radiko_client, Arc::clone(&program)).await?;
    Ok(())
}

async fn recording_for_live(
    radiko_client: &RadikoClient,
    program: Arc<ReserveProgram>,
) -> anyhow::Result<()> {
    let program_info = program.get_info();
    let on_air_duration = program.on_air_duration();
    trace!(
        "recording for live: on_air_duration_secs: {:#?}, program: {}",
        on_air_duration, program_info
    );

    let media_list_url = radiko_client
        .media_list_url_for_live(&program.station_id())
        .await?;
    let stream_handler = StreamHandler::new(reqwest::Client::new(), media_list_url);
    info!("start recording for live: {}", program_info);
    stream_handler
        .start_recording(
            program.output_dir(),
            &program.output_filename(),
            Duration::from_secs(on_air_duration.0),
        )
        .await
}
