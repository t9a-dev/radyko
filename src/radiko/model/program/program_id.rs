use std::fmt::{self, Display};

use anyhow::{Context, bail};
use jiff::{ToSpan, Zoned, civil::DateTime};
use serde::Deserialize;
use tracing::error;

use crate::{RADYKO_TZ_NAME, radiko::model::program::jst_datetime};

#[derive(Debug, Clone, Hash, PartialEq, Eq, Deserialize)]
/// 番組情報が一意になる値を返す。録音予約済み判定に利用。
pub struct ProgramId(
    StationId,
    #[serde(with = "jst_datetime")] StartAt,
    #[serde(with = "jst_datetime")] EndAt,
);

impl Display for ProgramId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", self.0, self.1.display(), self.2.display())
    }
}

impl ProgramId {
    pub fn new(station_id: StationId, start_at: StartAt, end_at: EndAt) -> Self {
        Self(station_id, start_at, end_at)
    }

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
                    StartAt::from_str(start_at)?,
                    EndAt::from_str(end_at)?,
                ))
            })
            .collect()
    }

    pub fn station_id(&self) -> &StationId {
        &self.0
    }

    pub fn start_at(&self) -> &StartAt {
        &self.1
    }

    pub fn end_at(&self) -> &EndAt {
        &self.2
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Deserialize)]
pub struct StationId(String);

impl Display for StationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl StationId {
    pub fn new(station_id: String) -> Self {
        Self(station_id)
    }

    pub fn get(self) -> String {
        self.0
    }
}

pub trait RadykoDateTime {
    fn new(zoned: Zoned) -> Self
    where
        Self: Sized;

    fn from_str(s: &str) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self::new(
            DateTime::strptime(Self::format_str(), s)
                .with_context(|| {
                    format!(
                        "failed string to datetime. str: {s}, format: {}",
                        Self::format_str()
                    )
                })?
                .in_tz(RADYKO_TZ_NAME)?,
        ))
    }

    fn format_str() -> &'static str {
        "%Y%m%d%H%M%S"
    }

    fn date(&self) -> Zoned;

    /// [`Self::format_str`] が返す形式でこの値をフォーマットする Display adapter を返します。
    fn display(&self) -> RadykoDateTimeDisplay<'_, Self>
    where
        Self: Sized,
    {
        RadykoDateTimeDisplay(self)
    }

    fn format(&self, format: &str) -> String {
        self.date().strftime(format).to_string()
    }
}

pub struct RadykoDateTimeDisplay<'a, T: ?Sized>(&'a T);

impl<T: RadykoDateTime + ?Sized> Display for RadykoDateTimeDisplay<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.format(T::format_str()))
    }
}
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct StartAt(Zoned);

impl RadykoDateTime for StartAt {
    fn new(end_at: Zoned) -> Self {
        Self(end_at)
    }

    fn date(&self) -> Zoned {
        self.0.clone()
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct EndAt(Zoned);

impl RadykoDateTime for EndAt {
    fn new(end_at: Zoned) -> Self {
        Self(end_at)
    }

    fn date(&self) -> Zoned {
        self.0.clone()
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct SeekStartAt(Zoned);

impl RadykoDateTime for SeekStartAt {
    fn new(seek_start_at: Zoned) -> Self {
        Self(seek_start_at)
    }

    fn date(&self) -> Zoned {
        self.0.clone()
    }
}

impl SeekStartAt {
    pub fn calculate_seek_start_times(mut start_at: StartAt, end_at: EndAt) -> Vec<Self> {
        if end_at.date() <= start_at.date() {
            error!("end must be greater than start");
            return vec![];
        }

        let mut times = vec![];
        while start_at.date() < end_at.date() {
            times.push(SeekStartAt(start_at.0.clone()));
            // radikoのHLSにおいて、medialist_urlには5秒の音声セグメントが3つ入っている
            // 1リクエストに15秒分の音声セグメントが対応しているので15秒枚のSeekStartTimeを計算
            let Ok(next_time) = start_at.date().checked_add(15.seconds()) else {
                break;
            };
            start_at = StartAt::new(next_time);
        }

        times
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        radiko::model::program::{EndAt, RadykoDateTime, SeekStartAt, StartAt},
        test_helper::parse_datetime_in_tz_tokyo,
    };

    #[test]
    fn calculate_seek_start_times_test() {
        let start = parse_datetime_in_tz_tokyo("2000-01-01 00:00:00");
        let end = parse_datetime_in_tz_tokyo("2000-01-01 00:01:00");
        let mut seek_start_times =
            SeekStartAt::calculate_seek_start_times(StartAt::new(start), EndAt::new(end));
        seek_start_times.sort();

        assert_eq!(seek_start_times[0].display().to_string(), "20000101000000");
        assert_eq!(seek_start_times[1].display().to_string(), "20000101000015");
        assert_eq!(seek_start_times[2].display().to_string(), "20000101000030");
        assert_eq!(
            seek_start_times[3].display().to_string(),
            "20000101000045".to_string()
        );

        assert_eq!(seek_start_times.len(), 4);
    }
}
