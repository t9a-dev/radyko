use std::{collections::HashMap, path::PathBuf};

use jiff::Zoned;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::app::{program_selector::ProgramSelector, types::Station};

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
pub struct RadykoConfigKeywords(HashMap<Station, Vec<String>>);

impl RadykoConfigKeywords {
    pub fn new(keywords: HashMap<Station, Vec<String>>) -> Self {
        Self(keywords)
    }

    pub fn into_program_selectors(self) -> Vec<ProgramSelector> {
        self.0
            .into_iter()
            .map(|(station_id, keywords)| {
                ProgramSelector::new_keyword_selector(station_id, keywords)
            })
            .collect()
    }
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RadykoConfigRules(HashMap<Station, Vec<String>>);

impl RadykoConfigRules {
    pub fn new(rules: HashMap<Station, Vec<String>>) -> Self {
        Self(rules)
    }

    pub fn try_into_program_selectors(
        self,
        now: Option<Zoned>,
    ) -> anyhow::Result<Vec<ProgramSelector>> {
        Ok(self
            .0
            .into_iter()
            .flat_map(|(station_id, cron_list)| {
                cron_list
                    .into_iter()
                    .flat_map(|cron| {
                        ProgramSelector::new_rule_selector(station_id.clone(), cron, now.clone())
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>())
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RadykoConfig {
    pub recording: RecordingConfig,
    keywords: Option<RadykoConfigKeywords>,
    rules: Option<RadykoConfigRules>,
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

    pub fn collect_program_selectors(&self) -> anyhow::Result<Vec<ProgramSelector>> {
        let mut selectors = Vec::new();
        if self.keywords.is_none() && self.rules.is_none() {
            warn!("keywords and rules config is empty");
            return Ok(selectors);
        }

        match self.keywords.clone() {
            Some(keywords) => selectors.extend(keywords.into_program_selectors()),
            None => info!("keywords not found."),
        }
        match self.rules.clone() {
            Some(rules) => selectors.extend(rules.try_into_program_selectors(None)?),
            None => info!("rules not found."),
        }
        Ok(selectors)
    }
}
