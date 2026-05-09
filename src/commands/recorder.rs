use crate::{
    app::{
        hls::StreamHandler,
        program_reserver::ProgramReserver,
        program_resolver,
        state::{AppState, RecorderState},
        types::RecordingEvent,
        utils::{self, Utils},
    },
    cli::RecorderArgs,
    commands::common::{collect_program_selectors, resolve_programs},
};
use std::{
    fs,
    io::{BufWriter, Write},
    sync::Arc,
    time::Duration,
};
use tracing::{debug, error, info};

// 構造体の内容がまるごと表示されてノイズになるので出力対象外にしている。skip(recorder_state)
#[tracing::instrument(name = "cli_command_recorder" skip(args))]
pub async fn run(args: RecorderArgs) -> anyhow::Result<()> {
    let app_state = Arc::new(AppState::build_from_recorder_args(args.clone()).await?);
    Utils::is_writable_output_dir(&app_state.output_dir().to_string_lossy());

    // 録音ファイル出力ディレクトリ直下に録音予約管理ファイルを配置することでコンテナ環境でも追加の設定無しに永続化される
    let reserved_state_file_path = app_state.output_dir().join("reserved_programs");
    let recorder_state = Arc::new(RecorderState::new(
        Arc::clone(&app_state),
        reserved_state_file_path,
    ));
    let mut reserve_schedule_update_interval = tokio::time::interval(
        tokio::time::Duration::from_secs(recorder_state.schedule_update_interval_secs()),
    );
    // 最初のtick()は即座に完了する
    reserve_schedule_update_interval.tick().await;

    loop {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        match reserve(Arc::clone(&recorder_state), tx).await {
            Ok(_) => info!("recorder run success"),
            Err(e) => error!("recorder error: {:#?}", e),
        }
        let _ = recording_event_handler(Arc::clone(&recorder_state), rx).await;
        let shared_recorder_state = Arc::clone(&recorder_state);
        if let Err(e) = download_timefree_programs(shared_recorder_state).await {
            error!("timefree download error: {:#?}", e);
        };
        reserve_schedule_update_interval.tick().await;
        if let Err(e) = recorder_state.reload_config(args.config.config_path.clone()) {
            error!("error reload config: {:#?}", e);
        }
    }
}

async fn reserve(
    recorder_state: Arc<RecorderState>,
    tx: tokio::sync::mpsc::Sender<RecordingEvent>,
) -> anyhow::Result<()> {
    info!("local now: {}", utils::Utils::formated_now_in_tz_tokyo()?);
    let program_selectors = collect_program_selectors(
        &recorder_state
            .config()
            .read()
            .expect("recorder_state config RwLock poisoned"),
    )?;
    let programs = resolve_programs(recorder_state.app_state(), program_selectors).await?;

    // println!(): programsをforで回しながらprintln!()するとprintln!()のたびにstdioをロックする。
    // writeln!(): 一度stdioをロックして、出力内容をbufferに書き溜めて最後に一度表示するので効率が良い。
    // programsは100も行かないので記述量の増加を回収できないと考えるが、学習のためということで良しとする。
    let stdio = std::io::stdout();
    let mut writer = BufWriter::new(stdio.lock());

    let program_reserver = ProgramReserver::new(
        recorder_state.app_state().radiko_client.clone(),
        recorder_state.recording_config(),
    );
    let reserved_programs = recorder_state.add_reserve_programs(programs);
    for program in reserved_programs {
        let add_reserve_program_info = format!("add reserve: {}", program.get_info());
        debug!(add_reserve_program_info);
        writeln!(writer, "{}", add_reserve_program_info)?;
        program_reserver.reserve(program, tx.clone()).await?;
    }

    writer.flush()?;
    Ok(())
}

async fn download_timefree_programs(recorder_state: Arc<RecorderState>) -> anyhow::Result<()> {
    let program_ids = recorder_state.collect_aired_program_ids(None)?;
    let radiko_client = &recorder_state.app_state().radiko_client;
    let timefree_programs =
        program_resolver::resolve_program_id(radiko_client, program_ids).await?;
    if timefree_programs.is_empty() {
        info!("timefree programs empty");
        return Ok(());
    }

    let radiko_client = recorder_state
        .app_state()
        .radiko_client
        .refresh_auth()
        .await?;
    let stream_handler = StreamHandler::new(reqwest::Client::new());
    for program in timefree_programs {
        let media_list_urls = radiko_client
            .clone()
            .collect_timefree_medialist_urls(
                program.station_id.clone(),
                program.start_time.clone(),
                program.end_time.clone(),
            )
            .await?;
        let recorded_file_path = stream_handler
            .download_timefree_program(
                media_list_urls,
                program.output_dir(recorder_state.app_state().output_dir()),
                &program.output_filename(),
            )
            .await?;
        let recorded_file = fs::File::open(recorded_file_path)?;
        StreamHandler::verify_recorded_file_bytes(
            recorded_file.metadata()?,
            Duration::from_secs(program.on_air_duration().0),
        )?;
        recorder_state.remove_reserved_program(program.program_id())?
    }

    Ok(())
}

async fn recording_event_handler(
    recorder_state: Arc<RecorderState>,
    mut rx: tokio::sync::mpsc::Receiver<RecordingEvent>,
) -> anyhow::Result<()> {
    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                RecordingEvent::Done(program_id) => {
                    // 録音処理に成功したので録音予約情報を削除
                    if let Err(e) = recorder_state.remove_reserved_program(program_id) {
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
