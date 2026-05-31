use std::{path::PathBuf, sync::Arc};

use tracing::{Instrument, error, info, trace};

use crate::{
    application::{
        hls::StreamHandler, port::RadikoClient, state::RecorderState, types::RecordingEvent,
    },
    domain::program::{Program, RecordingDurationBuffers},
};

#[derive(Clone)]
pub struct ReserveProgramUseCase {
    radiko_client: Arc<dyn RadikoClient>,
    recorder_state: Arc<RecorderState>,
}

impl ReserveProgramUseCase {
    pub fn new(radiko_client: Arc<dyn RadikoClient>, recorder_state: Arc<RecorderState>) -> Self {
        Self {
            radiko_client,
            recorder_state,
        }
    }

    pub async fn exec(
        self,
        programs: Vec<Program>,
        output_root_dir: PathBuf,
        recording_duration_buffers: RecordingDurationBuffers,
    ) -> anyhow::Result<()> {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        // 録音情報の永続化
        let reserved_programs = self.recorder_state.add_reserve_programs(programs);

        for program in reserved_programs {
            // 番組毎に録音予約をspawn
            self.clone()
                .spawn_reserve(
                    program,
                    output_root_dir.clone(),
                    recording_duration_buffers,
                    tx.clone(),
                )
                .await?;
        }

        self.spawn_recording_event_handler(rx).await?;

        Ok(())
    }

    async fn spawn_reserve(
        self,
        program: Program,
        output_root_dir: PathBuf,
        buffer: RecordingDurationBuffers,
        tx: tokio::sync::mpsc::Sender<RecordingEvent>,
    ) -> anyhow::Result<()> {
        // 録音予約はspawnしてawaitせず、そのまま任せる。
        tokio::spawn(
            async move {
                let program = Arc::new(program);
                program.wait_for_live_on_air(&buffer.start_buffer()).await;
                let _ = self
                    .radiko_client
                    .refresh_auth()
                    .await
                    .map_err(|e| error!("failed refresh auth radiko client: {e:#?}"));

                if let Err(e) = std::fs::create_dir_all(program.output_dir(output_root_dir.clone()))
                {
                    error!("create recording dir error: {:#?}", e)
                };
                match self
                    .start_recording_for_live(
                        Arc::clone(&program),
                        output_root_dir.clone(),
                        &buffer,
                    )
                    .await
                {
                    Ok(_) => {
                        let _ = tx.send(RecordingEvent::Done(program.program_id())).await;
                    }
                    Err(e) => {
                        let _ = tx.send(RecordingEvent::Fail(program.program_id())).await;
                        error!("recording error: {:#?}", e);
                    }
                };
            }
            .in_current_span(),
        );

        Ok(())
    }

    async fn start_recording_for_live(
        self,
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

        let media_playlist_url = program
            .media_playlist_url_for_live(self.radiko_client)
            .await?;
        let stream_handler = StreamHandler::new(reqwest::Client::new());
        info!("start recording for live: {}", program_info);
        stream_handler
            .start_recording(
                media_playlist_url,
                program.output_dir(output_root_dir),
                &program.output_filename(),
                on_air_duration,
            )
            .await
    }

    async fn spawn_recording_event_handler(
        self,
        mut rx: tokio::sync::mpsc::Receiver<RecordingEvent>,
    ) -> anyhow::Result<()> {
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    RecordingEvent::Done(program_id) => {
                        // 録音処理に成功したので録音予約情報を削除
                        if let Err(e) = self.recorder_state.remove_reserved_program(program_id) {
                            error!("failed remove reserved program: {:#?}", e);
                        };
                    }
                    RecordingEvent::Fail(program_id) => {
                        // 録音予約時点で録音予約は永続化されており、録音成功時に録音情報が削除される
                        // タイムフリーダウンロード処理成功時点で永続化してある録音予約情報が削除される
                        // ここでは録音予約情報を削除せず、ログだけ出力する
                        info!("リアルタイム録音処理に失敗: {}", program_id);
                    }
                }
            }
        });

        Ok(())
    }
}
