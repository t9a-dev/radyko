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

pub async fn start(
    radiko_client: RadikoClient,
    program: Arc<ReserveProgram>,
) -> anyhow::Result<()> {
    recording_with_stream_handler(&radiko_client, Arc::clone(&program)).await?;
    Ok(())
}

async fn recording_with_stream_handler(
    radiko_client: &RadikoClient,
    program: Arc<ReserveProgram>,
) -> anyhow::Result<()> {
    let program_info = program.get_info();
    let on_air_duration = program.on_air_duration();
    trace!(
        "stream handler recording: on_air_duration_secs: {:#?}, program: {}",
        on_air_duration, program_info
    );

    let media_list_url = radiko_client.media_list_url(&program.station_id()).await?;
    let stream_handler = StreamHandler::new(reqwest::Client::new(), media_list_url);
    info!("start recording with stream handler: {}", program_info);
    stream_handler
        .start_recording(
            program.output_dir(),
            &program.output_filename(),
            Duration::from_secs(on_air_duration.0),
        )
        .await
}
