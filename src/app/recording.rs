use std::{sync::Arc, time::Duration};

use tracing::{Instrument, error, info, trace};

use crate::{
    app::{
        ffmpeg_builder::{FfmpegBuilder, LogLevel},
        hls::StreamHandler,
        program_reserver::ReserveProgram,
        utils::Utils,
    },
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
    retry_policy: RetryWithBackOffPolicy,
) -> anyhow::Result<()> {
    let recorging_with_stream_handler_result = Utils::retry_with_backoff(
        || {
            async { recording_with_stream_handler(&radiko_client, Arc::clone(&program)).await }
                .in_current_span()
        },
        retry_policy.max_attempts,
        retry_policy.base_delay,
        retry_policy.max_delay,
    )
    .await;

    // stream handler録音できなければ、ffmpegを利用した録音処理にフェイルオーバーする
    if let Err(e) = recorging_with_stream_handler_result {
        error!("recording with stream handler failed error: {:#?}", e);
        Utils::retry_with_backoff(
            || {
                async { recording_with_ffmpeg(&radiko_client, Arc::clone(&program)).await }
                    .in_current_span()
            },
            retry_policy.max_attempts,
            retry_policy.base_delay,
            retry_policy.max_delay,
        )
        .await?
    }

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

async fn recording_with_ffmpeg(
    radiko_client: &RadikoClient,
    program: Arc<ReserveProgram>,
) -> anyhow::Result<()> {
    let program_info = program.get_info();
    let on_air_duration = program.on_air_duration();
    trace!(
        "ffmpeg recording: on_air_duration_secs: {:#?}, program: {}",
        on_air_duration, program_info,
    );

    let recorder = FfmpegBuilder::new()
        .loglevel(LogLevel::Error)
        .header("X-Radiko-Authtoken", &radiko_client.auth_token().await)
        .input(radiko_client.stream_url(&program.station_id()).await)
        .for_hls_recording(on_air_duration)
        .output(program.output_full_path().to_string_lossy())
        .build()
        .await?;
    info!("start recording with ffmpeg: {}", program_info);

    let status = recorder.write().await.wait().await?;
    if status.success() {
        info!("sucess recording: {}", &program_info);
    } else {
        error!("error recording: {:#?}", &program);
    }

    Ok(())
}
