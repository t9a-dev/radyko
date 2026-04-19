use std::io::{self, BufWriter, Write};

use secrecy::ExposeSecret;

use crate::{app::credential::RadikoCredential, cli::SearchArgs, radiko::RadikoClient};

#[tracing::instrument(name = "cli_command_search")]
pub async fn run(args: SearchArgs) -> anyhow::Result<()> {
    let radiko_credential = RadikoCredential::load_credential();
    let radiko_client = match radiko_credential {
        Some(c) => {
            RadikoClient::new_area_free(c.email_address.expose_secret(), c.password.expose_secret())
                .await?
        }
        None => RadikoClient::new().await?,
    };
    let programs = radiko_client
        .search_programs(args.keyword, args.station_id.as_deref())
        .await?
        .data;

    // println!(): programsをforで回しながらprintln!()するとprintln!()のたびにstdioをロックする。
    // writeln!(): 一度stdioをロックして、出力内容をbufferに書き溜めて最後に一度表示する方法が効率が良い。
    // programsは100も行かないので、記述量の増加を回収できるとは思わないが、学習のためということで良しとする。
    let stdio = io::stdout();
    let mut writer = BufWriter::new(stdio.lock());
    programs
        .into_iter()
        .for_each(|program| writeln!(writer, "{}", program.get_info()).unwrap());
    writer.flush()?;

    Ok(())
}
