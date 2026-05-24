use std::{
    io::{BufWriter, Write},
    sync::Arc,
};

use tracing::error;

use crate::{
    app::{state::AppState, utils::Utils},
    cli::RuleArgs,
    radiko::model::program::Programs,
};

#[tracing::instrument(name = "cli_command_rule")]
pub async fn run(args: RuleArgs) -> anyhow::Result<()> {
    let app_state = Arc::new(AppState::build_from_rule_args(args).await?);
    Utils::is_writable_output_dir(&app_state.output_dir().to_string_lossy());

    let program_selectors = app_state
        .config()
        .read()
        .expect("app_state config RwLock poisoned")
        .collect_program_selectors()?;
    let programs =
        Programs::resolve_selectors(app_state.radiko_client(), program_selectors).await?;

    // println!(): programsをforで回しながらprintln!()するとprintln!()のたびにstdioをロックする。
    // writeln!(): 一度stdioをロックして、出力内容をbufferに書き溜めて最後に一度表示する方法が効率が良い。
    // programsは100も行かないので記述量の増加を回収できないと考えるが、学習のためということで良しとする。
    let stdio = std::io::stdout();
    let mut writer = BufWriter::new(stdio.lock());
    if let Err(e) = programs
        .into_iter()
        .try_for_each(|program| writeln!(writer, "{}", program.info()))
    {
        error!("failed wirte program info to stdout: {:#?}", e);
    };
    writer.flush()?;

    Ok(())
}
