use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::app::types::Station;

pub const EXAMPLE_CONFIG: &str = r#"# src/app/config.rs

[recording]
# 録音ファイルの保存先
# docker-composeで利用するときのコンテナ内でのパスに注意(カレントディレクトリ以下で指定しておけば問題にならない)
output_dir = "./recorded"
# 録音対象番組の取得間隔
schedule_update_interval_secs = 3600 

# 番組開始・終了時間に対する録音開始・終了のバッファ時間
[recording.duration_buffer_secs]
# 番組開始時間に対してX秒前に録音開始
start = 30
# 番組終了時間に対してX秒後に録音終了
# radikoは遅延があるので終了時間側のバッファを長めにとっておくとよさそう
end = 90

[keywords]
# 放送局の指定なし
nationwide = [
  "トム・ブラウン",
  "トムブラウン",
]

"FBC" = [
  "にゃんこスターのきらりん",
]

# cron形式による指定
# https://docs.rs/cron/latest/cron/#example
[rules]
LFR = [
  "0 0 0 * * Mon-Sat",
  "0 0 1 * * Tue-Sat",
  "0 0 3 * * Mon-Sat",
]
TBS = [
  "0 0 0 * * 3,4,6",
  "0 0 1 * * 4-7",
]"#;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RecordingConfig {
    pub output_dir: PathBuf,
    pub schedule_update_interval_secs: u64,
    pub duration_buffer_secs: Option<RecordingDurationBufferConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RecordingDurationBufferConfig {
    pub start: u64,
    pub end: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RadykoConfigKeywords(pub HashMap<Station, Vec<String>>);
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RadykoConfigRules(pub HashMap<Station, Vec<String>>);

#[derive(Debug, Deserialize, Serialize)]
pub struct RadykoConfig {
    pub recording: RecordingConfig,
    pub keywords: Option<RadykoConfigKeywords>,
    pub rules: Option<RadykoConfigRules>,
}
impl RadykoConfig {
    pub fn parse<R: std::io::Read>(mut reader: R) -> anyhow::Result<Self> {
        let mut buf = String::new();
        let _ = reader.read_to_string(&mut buf);

        Ok(toml::from_str::<Self>(&buf)?)
    }

    pub fn parse_from_path(config_path: PathBuf) -> anyhow::Result<Self> {
        let reader = std::fs::File::open(config_path)?;
        Self::parse(reader)
    }
}
