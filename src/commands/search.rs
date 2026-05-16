use std::io::{self, BufWriter, Write};

use tracing::error;

use crate::{cli::SearchArgs, radiko::RadikoClient};

#[tracing::instrument(name = "cli_command_search")]
pub async fn run(args: SearchArgs) -> anyhow::Result<()> {
    let radiko_client = RadikoClient::new(None).await?;
    let programs = radiko_client
        .search_programs(args.keyword, args.station_id.as_deref())
        .await?
        .data;

    // println!(): programsをforで回しながらprintln!()するとprintln!()のたびにstdioをロックする。
    // writeln!(): 一度stdioをロックして、出力内容をbufferに書き溜めて最後に一度表示する方法が効率が良い。
    // programsは100も行かないので、記述量の増加を回収できるとは思わないが、学習のためということで良しとする。
    let stdio = io::stdout();
    let mut writer = BufWriter::new(stdio.lock());
    if let Err(e) = programs
        .into_iter()
        .try_for_each(|program| writeln!(writer, "{}", program.get_info()))
    {
        error!("failed wirte program info to stdout: {:#?}", e);
    };
    writer.flush()?;

    Ok(())
}
