use std::path::PathBuf;

use anyhow::bail;
use clap::{Args, Parser, Subcommand};

use crate::app::config::{self};
use crate::commands::{recorder, rule, search};
use crate::telemetry::{init_telemetry, send_otel_connectivity_check};

#[derive(Parser)]
#[command(
    name = "radyko(らでぃこ)",
    version,
    about = "非公式radikoクライアント（番組の録音・検索）"
)]
pub struct Cli {
    /// Log level: error, warn, info, debug, trace
    #[arg(short('l'), long, default_value = "info")]
    pub log_level: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// 指定した設定ファイルに基づいて録音プログラムを起動 ex: radyko recorder -c radyko.toml
    Recorder(RecorderArgs),
    /// 指定した設定ファイルのルールに一致する番組の確認 ex: radyko rule -c radyko.toml
    Rule(RuleArgs),
    /// 番組の検索 ex: radyko search -k "オールナイトニッポン" -s "LFR"
    Search(SearchArgs),
    /// 設定ファイル例の出力 ex: radyko init > example.radyko.toml
    Init,
}

#[derive(Args, Debug, Clone)]
pub struct ConfigArgs {
    #[arg(short, long)]
    /// config path
    pub config_path: PathBuf,
}

#[derive(Args, Debug, Clone)]
pub struct RecorderArgs {
    #[command(flatten)]
    /// config path
    pub config: ConfigArgs,
}

#[derive(Args, Debug, Clone)]
pub struct RuleArgs {
    #[command(flatten)]
    /// config path
    pub config: ConfigArgs,
}

#[derive(Args, Debug, Clone)]
pub struct SearchArgs {
    #[arg(short('k'), long)]
    /// keyword
    pub keyword: String,

    #[arg(short('s'), long)]
    /// keyword
    pub station_id: Option<String>,
}

pub async fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    init_telemetry("radyko_cli", cli.log_level.as_deref());
    send_otel_connectivity_check();

    match cli.command {
        Some(Commands::Recorder(args)) => {
            let _ = recorder::run(args).await;
            Ok(())
        }
        Some(Commands::Rule(args)) => rule::run(args).await,
        Some(Commands::Search(args)) => search::run(args).await,
        Some(Commands::Init) => {
            println!("{}", config::EXAMPLE_CONFIG);
            Ok(())
        }
        None => bail!("no command provided. try `--help` "),
    }
}
