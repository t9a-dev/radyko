use std::{path::PathBuf, sync::Arc};

use anyhow::bail;
use clap::{Args, Parser, Subcommand};
use tracing::{error, info};

use crate::app::config::{self};
use crate::app::state::{AppState, RecorderState};
use crate::app::utils::Utils;
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
            let app_state = Arc::new(AppState::build_from_recorder_args(args.clone()).await?);
            Utils::is_writable_output_dir(app_state.output_dir().to_str().unwrap());

            let recorder_state = Arc::new(RecorderState::new(Arc::clone(&app_state)));
            loop {
                let mut reserve_schedule_update_interval =
                    tokio::time::interval(tokio::time::Duration::from_secs(
                        recorder_state.schedule_update_interval_secs(),
                    ));
                // 最初のtick()は即座に完了する
                reserve_schedule_update_interval.tick().await;

                match recorder::run(Arc::clone(&recorder_state)).await {
                    Ok(_) => info!("recorder run success"),
                    Err(e) => error!("recorder error: {:#?}", e),
                }
                reserve_schedule_update_interval.tick().await;
                recorder_state.reload_config(args.config.config_path.clone())?;
            }
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
