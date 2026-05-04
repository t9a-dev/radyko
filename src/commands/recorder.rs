use crate::{
    app::{
        program_reserver::ProgramReserver,
        state::{AppState, RecorderState},
        utils::Utils,
    },
    cli::RecorderArgs,
    commands::common::{collect_program_selectors, resolve_programs},
};
use std::{
    io::{BufWriter, Write},
    path::PathBuf,
    str::FromStr,
    sync::Arc,
};
use tracing::{debug, error, info};

// 構造体の内容がまるごと表示されてノイズになるので出力対象外にしている。skip(recorder_state)
#[tracing::instrument(name = "cli_command_recorder" skip(args))]
pub async fn run(args: RecorderArgs) {
    let app_state = Arc::new(
        AppState::build_from_recorder_args(args.clone())
            .await
            .unwrap(),
    );
    Utils::is_writable_output_dir(app_state.output_dir().to_str().unwrap());

    let reserved_state_file_path = PathBuf::from_str("./reserved_programs").unwrap();
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
        match reserve(Arc::clone(&recorder_state)).await {
            Ok(_) => info!("recorder run success"),
            Err(e) => error!("recorder error: {:#?}", e),
        }
        reserve_schedule_update_interval.tick().await;
        if let Err(e) = recorder_state.reload_config(args.config.config_path.clone()) {
            error!("error reload config: {:#?}", e);
        }
    }
}

async fn reserve(recorder_state: Arc<RecorderState>) -> anyhow::Result<()> {
    info!("local now: {}", chrono::Local::now());
    let program_selectors = collect_program_selectors(&recorder_state.config().read().unwrap())?;
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
    for program in programs {
        if !recorder_state.insert_reserved_program_id(&program) {
            debug!("skip reserved program: {}", program.get_info());
            continue;
        }
        let add_reserve_program_info = format!("add reserve: {}", program.get_info());
        debug!(add_reserve_program_info);
        writeln!(writer, "{}", add_reserve_program_info)?;
        program_reserver.reserve(program).await?;
    }

    writer.flush()?;
    Ok(())
}
