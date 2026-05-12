use std::{path::PathBuf, sync::Arc, time::Duration};

use jiff::Zoned;
use tracing::{Instrument, error, trace};

use crate::{
    app::{
        config::{RecordingConfig, RecordingDurationBufferConfig},
        recording::{self},
        types::RecordingEvent,
    },
    model::program::{Program, Seconds},
    radiko::RadikoClient,
};

#[derive(Debug, Clone, Copy)]
struct RecordingDurationBuffer {
    pub start: Seconds,
    pub end: Seconds,
}

impl RecordingDurationBuffer {
    pub fn new(config: Option<RecordingDurationBufferConfig>) -> Self {
        match config {
            Some(config) => Self {
                start: Seconds(config.start),
                end: Seconds(config.end),
            },
            None => Self::default(),
        }
    }
}

impl Default for RecordingDurationBuffer {
    fn default() -> Self {
        Self {
            start: Seconds(0),
            end: Seconds(0),
        }
    }
}

#[derive(Debug)]
pub struct ReserveProgram {
    program: Program,
    output_root_dir: PathBuf,
    start_buffer: Seconds,
    end_buffer: Seconds,
}

impl ReserveProgram {
    pub fn new(
        program: Program,
        output_root_dir: PathBuf,
        recording_duration_buffer: Option<RecordingDurationBufferConfig>,
    ) -> Self {
        let duration_buffer = RecordingDurationBuffer::new(recording_duration_buffer);
        Self {
            program: program.clone(),
            output_root_dir,
            start_buffer: duration_buffer.start,
            end_buffer: duration_buffer.end,
        }
    }

    pub fn get_info(&self) -> String {
        self.program.get_info()
    }

    /// 番組開始時間までの秒数を計算してsleep
    pub async fn wait_for_on_air(&self) {
        let wait_for_on_air_secs = self.to_on_air_duration_with_buffer(None).0;
        trace!("wait for on air secs: {}", wait_for_on_air_secs);
        tokio::time::sleep(Duration::from_secs(wait_for_on_air_secs)).await;
    }

    pub fn on_air_duration(&self) -> Seconds {
        self.program
            .on_air_duration_with_buffer(self.start_buffer, self.end_buffer)
    }

    pub fn station_id(&self) -> String {
        self.program.station_id.clone()
    }

    pub fn output_full_path(&self) -> PathBuf {
        self.output_dir().join(self.output_filename())
    }

    pub fn output_dir(&self) -> PathBuf {
        self.program.output_dir(self.output_root_dir.clone())
    }

    pub fn output_filename(&self) -> String {
        self.program.output_filename()
    }

    fn to_on_air_duration_with_buffer(&self, now: Option<Zoned>) -> Seconds {
        Seconds(
            self.program
                .to_on_air_duration(now)
                .0
                .saturating_sub(self.start_buffer.0),
        )
    }
}

#[derive(Debug, Clone)]
pub struct ProgramReserver {
    inner: Arc<ProgramReserverRef>,
}

#[derive(Debug)]
struct ProgramReserverRef {
    radiko_client: RadikoClient,
    config: RecordingConfig,
}

impl ProgramReserver {
    pub fn new(radiko_client: RadikoClient, config: RecordingConfig) -> Self {
        Self {
            inner: Arc::new(ProgramReserverRef {
                radiko_client,
                config,
            }),
        }
    }

    #[tracing::instrument(name = "recorder_reserve" skip(self,program,tx))]
    pub async fn reserve(
        &self,
        program: Program,
        tx: tokio::sync::mpsc::Sender<RecordingEvent>,
    ) -> anyhow::Result<()> {
        let program = Arc::new(ReserveProgram::new(
            program,
            self.inner.config.output_dir.clone(),
            self.inner.config.duration_buffer_secs.clone(),
        ));

        // 録音予約はspawnしてawaitせず、そのまま任せる。
        let this = self.clone();
        tokio::spawn(
            async move {
                program.wait_for_on_air().await;
                let refreshed_radiko_client = match this.inner.radiko_client.refresh_auth().await {
                    Ok(refreshed_client) => refreshed_client,
                    Err(e) => {
                        error!("failed refresh radiko client: {:#?}", e);
                        // トークンのリフレッシュに失敗したら、そのまま現状のクライアントを使う
                        this.inner.radiko_client.clone()
                    }
                };
                if let Err(e) = tokio::fs::create_dir_all(program.output_dir()).await {
                    error!("create recording dir error: {:#?}", e)
                };
                match recording::start_for_live(refreshed_radiko_client, program.clone()).await {
                    Ok(_) => {
                        let _ = tx
                            .send(RecordingEvent::Done(program.program.program_id()))
                            .await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(RecordingEvent::Fail(program.program.program_id()))
                            .await;
                        error!("recording error: {:#?}", e);
                    }
                };
            }
            .in_current_span(),
        );

        Ok(())
    }
}
