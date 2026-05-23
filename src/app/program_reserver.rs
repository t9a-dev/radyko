use std::{path::PathBuf, sync::Arc};

use tracing::{Instrument, error};

use crate::{
    app::{
        recording::{self},
        types::RecordingEvent,
    },
    radiko::RadikoClient,
    radiko::model::program::{Program, RecordingDurationBuffers},
};

#[derive(Debug, Clone)]
pub struct ProgramReserver {
    inner: Arc<ProgramReserverRef>,
}

#[derive(Debug)]
struct ProgramReserverRef {
    radiko_client: RadikoClient,
    output_root_dir: PathBuf,
}

impl ProgramReserver {
    pub fn new(radiko_client: RadikoClient, output_root_dir: PathBuf) -> Self {
        Self {
            inner: Arc::new(ProgramReserverRef {
                radiko_client,
                output_root_dir,
            }),
        }
    }

    #[tracing::instrument(name = "recorder_reserve" skip(self,buffer,program,tx))]
    pub async fn reserve(
        &self,
        program: Program,
        buffer: RecordingDurationBuffers,
        tx: tokio::sync::mpsc::Sender<RecordingEvent>,
    ) -> anyhow::Result<()> {
        // 録音予約はspawnしてawaitせず、そのまま任せる。
        let this = self.clone();
        tokio::spawn(
            async move {
                let program = Arc::new(program);
                program.wait_for_live_on_air(&buffer.start_buffer()).await;
                let refreshed_radiko_client = match this.inner.radiko_client.refresh_auth().await {
                    Ok(refreshed_client) => refreshed_client,
                    Err(e) => {
                        error!("failed refresh radiko client: {:#?}", e);
                        // トークンのリフレッシュに失敗したら、そのまま現状のクライアントを使う
                        this.inner.radiko_client.clone()
                    }
                };
                if let Err(e) =
                    std::fs::create_dir_all(program.output_dir(this.inner.output_root_dir.clone()))
                {
                    error!("create recording dir error: {:#?}", e)
                };
                match recording::start_for_live(
                    &refreshed_radiko_client,
                    Arc::clone(&program),
                    this.inner.output_root_dir.clone(),
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
}
