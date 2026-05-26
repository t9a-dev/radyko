use std::{path::PathBuf, sync::Arc};

use tracing::{info, trace};

use crate::{
    application::{hls::StreamHandler, ports::RadikoClient},
    domain::program::{Program, RecordingDurationBuffers},
};

pub async fn start_for_live(
    radiko_client: Arc<dyn RadikoClient>,
    program: Arc<Program>,
    output_root_dir: PathBuf,
    buffers: &RecordingDurationBuffers,
) -> anyhow::Result<()> {
    let program_info = program.info();
    let on_air_duration = program.on_air_duration_for_live(buffers);
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
            on_air_duration,
        )
        .await
}
