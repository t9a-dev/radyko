use crate::{
    app::{program_reserver::ProgramReserver, state::RecorderState},
    commands::common::{collect_program_selectors, resolve_programs},
};
use std::{
    io::{BufWriter, Write},
    sync::Arc,
};
use tracing::{debug, info};

// 構造体の内容がまるごと表示されてノイズになるので出力対象外にしている。skip(recorder_state)
#[tracing::instrument(name = "cli_command_recorder" skip(recorder_state))]
pub async fn run(recorder_state: Arc<RecorderState>) -> anyhow::Result<()> {
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
        if !recorder_state.insert_reserved_program_id(program.program_id()) {
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
