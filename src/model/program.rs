use std::{
    fmt::{self, Display},
    path::PathBuf,
};

use anyhow::{Context, bail};
use jiff::{ToSpan, Zoned, civil::DateTime};
use radiko::api::endpoint::Endpoint;
use sanitise_file_name::sanitise;
use serde_derive::{Deserialize, Serialize};
use tracing::error;

use crate::{
    RADYKO_TZ_NAME,
    app::utils::Utils,
    radiko::{self, jst_datetime},
};

#[derive(Debug, Copy, Clone)]
pub struct Seconds(pub u64);

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
/// 番組情報が一意になる値を返す。録音予約済み判定に利用。
pub struct ProgramId(pub StationId, pub StartAt, pub EndAt);
impl Display for ProgramId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", self.0, self.1, self.2)
    }
}
impl ProgramId {
    pub fn parse_from_string(text: String) -> anyhow::Result<Vec<Self>> {
        text.lines()
            .skip_while(|line| line.is_empty())
            .map(|line| {
                let program_info = line.split_ascii_whitespace().take(3).collect::<Vec<_>>();
                let [station_id, start_at, end_at] = program_info.as_slice() else {
                    bail!("failed split program info: {:#?}", program_info)
                };
                Ok(ProgramId(
                    StationId(station_id.to_string()),
                    StartAt(Self::format_datetime(start_at)?),
                    EndAt(Self::format_datetime(end_at)?),
                ))
            })
            .collect()
    }

    fn format_datetime(s: &str) -> anyhow::Result<Zoned> {
        DateTime::strptime(Endpoint::DATETIME_FORMAT, s)?
            .in_tz(RADYKO_TZ_NAME)
            .with_context(|| format!("format_datetime str: {s}"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Programs {
    pub data: Vec<Program>,
}
impl Programs {
    pub fn find_program(self, start_at: Zoned) -> Option<Program> {
        self.data.into_iter().find(|p| p.start_time.eq(&start_at))
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct StationId(pub String);
impl Display for StationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct StartAt(pub Zoned);

impl Display for StartAt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.strftime(Endpoint::DATETIME_FORMAT))
    }
}
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct EndAt(pub Zoned);
impl Display for EndAt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.strftime(Endpoint::DATETIME_FORMAT))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Program {
    #[serde(with = "jst_datetime")]
    pub start_time: Zoned,
    #[serde(with = "jst_datetime")]
    pub end_time: Zoned,
    pub station_id: String,
    pub title: String,
    pub performer: String,
    // pub info: String,
    // pub description: String,
    // pub start_time_s: String,
    // pub end_time_s: String,
    // pub img: String,
}

impl Default for Program {
    fn default() -> Self {
        Self {
            start_time: Utils::now_in_tz_tokyo(),
            end_time: Utils::now_in_tz_tokyo(),
            station_id: "TEST".to_string(),
            title: "テスト番組名".to_string(),
            performer: "テスト出演者".to_string(),
            // start_time_s: Default::default(),
            // end_time_s: Default::default(),
            // info: Default::default(),
            // description: Default::default(),
            // img: Default::default(),
        }
    }
}

impl Program {
    pub fn new(start_time: Zoned, end_time: Zoned) -> Self {
        Self {
            start_time,
            end_time,
            ..Default::default()
        }
    }

    pub fn program_id(&self) -> ProgramId {
        ProgramId(
            StationId(self.station_id.clone()),
            StartAt(self.start_time.clone()),
            EndAt(self.end_time.clone()),
        )
    }

    pub fn get_info(&self) -> String {
        format!(
            "{}_{}_{}_{}",
            self.start_time, self.station_id, self.title, self.performer
        )
    }

    pub fn output_dir(&self, output_root_dir: PathBuf) -> PathBuf {
        output_root_dir.join(sanitise(&self.title))
    }

    pub fn output_filename(&self) -> String {
        sanitise(&format!(
            "{}_{}_{}_{}.aac",
            self.station_id,
            self.start_time.strftime("%Y%m%d_%H%M%S"),
            self.title,
            self.performer,
        ))
    }

    pub fn to_on_air_duration(&self, now: Option<Zoned>) -> Seconds {
        let now = now.unwrap_or(Utils::now_in_tz_tokyo());
        Seconds(
            self.start_time
                .duration_since(&now)
                .as_secs()
                .try_into()
                .unwrap_or(0),
        )
    }

    pub fn on_air_duration(&self) -> Seconds {
        Seconds(
            self.end_time
                .duration_since(&self.start_time)
                .as_secs()
                .try_into()
                .unwrap_or(0),
        )
    }

    pub fn on_air_duration_with_buffer(
        &self,
        start_buffer: Seconds,
        end_buffer: Seconds,
    ) -> Seconds {
        let (Seconds(start_buffer), Seconds(end_buffer)) = (start_buffer, end_buffer);

        // buffer分を減算放送開始時間より前倒しの時間を計算
        let start_time = self
            .start_time
            .checked_add(start_buffer.try_into().unwrap_or(0).seconds())
            .unwrap();
        // buffer分を加算して放送終了時間より後の時間を計算
        let end_time = self
            .end_time
            .checked_add(end_buffer.try_into().unwrap_or(0).seconds())
            .unwrap();

        Seconds(
            end_time
                .duration_since(&start_time)
                .as_secs()
                .try_into()
                .unwrap_or(0),
        )
    }
}

#[cfg(test)]
mod tests {

    use jiff::Unit;

    use crate::test_helper::parse_datetime_in_tz_tokyo;

    use super::*;

    #[test]
    fn on_air_duration_test() {
        let on_air_duration = 1.hours();
        let dummy_start_time = parse_datetime_in_tz_tokyo("2000-01-01 00:00:00");
        let dummy_end_time = dummy_start_time.checked_add(on_air_duration).unwrap();

        let program = Program::new(dummy_start_time, dummy_end_time);
        assert_eq!(
            on_air_duration.total(Unit::Second).unwrap() as u64,
            program.on_air_duration().0
        );
    }

    #[test]
    fn on_air_duration_with_buffer_test() {
        let on_air_duration = 1.hours();
        let dummy_start_time = parse_datetime_in_tz_tokyo("2000-01-01 00:00:00");
        let dummy_end_time = dummy_start_time.checked_add(on_air_duration).unwrap();

        let start_buffer_seconds = 1.minutes().get_seconds() as u64;
        let end_buffer_seconds = 1.minutes().get_seconds() as u64;
        let program = Program::new(dummy_start_time, dummy_end_time);
        assert_eq!(
            on_air_duration.total(Unit::Second).unwrap() as u64
                + start_buffer_seconds
                + end_buffer_seconds,
            program
                .on_air_duration_with_buffer(
                    Seconds(start_buffer_seconds),
                    Seconds(end_buffer_seconds)
                )
                .0
        );
    }
}
