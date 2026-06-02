use crate::{
    application::{
        port::RadikoClient,
        state::{AppState, RecorderState},
        usecase::{
            download_timefree::DownloadTimeFreeUseCase, reserve_program::ReserveProgramUseCase,
        },
        utils::Utils,
    },
    cli::RecorderArgs,
    domain::program::Programs,
    infrastructure::new_file_reserved_repository,
};
use std::{path::PathBuf, sync::Arc};
use tracing::{error, info};

// 構造体の内容がまるごと表示されてノイズになるので出力対象外にしている。skip(recorder_state)
#[tracing::instrument(name = "cli_command_recorder" skip(args))]
pub async fn run(args: RecorderArgs) -> anyhow::Result<()> {
    let app_state = Arc::new(AppState::build_from_recorder_args(args.clone()).await?);
    Utils::is_writable_output_dir(&app_state.output_dir().to_string_lossy());

    let mut reserve_schedule_update_interval = tokio::time::interval(
        tokio::time::Duration::from_secs(app_state.schedule_update_interval_secs()),
    );
    // 最初のtick()は即座に完了する
    reserve_schedule_update_interval.tick().await;

    // 録音ファイル出力ディレクトリ直下に録音予約管理ファイルを配置することでコンテナ環境でも追加の設定無しに永続化できる
    let reserved_state_file_path = app_state.output_dir().join("reserved_programs");
    let recorder_state = Arc::new(RecorderState::new(new_file_reserved_repository(
        reserved_state_file_path,
    )));
    loop {
        exec_reserve_programs(
            Arc::clone(&app_state.radiko_client()),
            Arc::clone(&app_state),
            Arc::clone(&recorder_state),
        )
        .await?;
        exec_download_timefree(
            Arc::clone(&app_state.radiko_client()),
            Arc::clone(&recorder_state),
            app_state.output_dir(),
        )
        .await?;

        reserve_schedule_update_interval.tick().await;
        if let Err(e) = app_state.reload_config(args.config.config_path.clone()) {
            error!("error reload config: {:#?}", e);
        }
    }
}

async fn exec_reserve_programs(
    radiko_client: Arc<dyn RadikoClient>,
    app_state: Arc<AppState>,
    recorder_state: Arc<RecorderState>,
) -> anyhow::Result<()> {
    let reserve_program_usecase =
        ReserveProgramUseCase::new(Arc::clone(&radiko_client), Arc::clone(&recorder_state));
    let program_selectors = app_state
        .config()
        .read()
        .expect("config RwLock poisoned")
        .collect_program_selectors()?;
    let programs =
        Programs::resolve_selectors(Arc::clone(&radiko_client), program_selectors).await?;

    match reserve_program_usecase
        .exec(
            programs,
            app_state.output_dir(),
            app_state.recording_duration_buffers(),
        )
        .await
    {
        Ok(_) => info!("recorder run success"),
        Err(e) => error!("recorder error: {:#?}", e),
    }
    Ok(())
}

async fn exec_download_timefree(
    radiko_client: Arc<dyn RadikoClient>,
    recorder_state: Arc<RecorderState>,
    output_root_dir: PathBuf,
) -> anyhow::Result<()> {
    let download_timefree_usecase = DownloadTimeFreeUseCase::new(
        reqwest::Client::new(),
        Arc::clone(&radiko_client),
        recorder_state,
    );
    if let Err(e) = download_timefree_usecase.exec(&output_root_dir).await {
        error!("timefree download error: {:#?}", e);
    };

    Ok(())
}
