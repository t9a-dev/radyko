use std::fmt::{self, Display};

use anyhow::{Context, bail};
use jiff::{ToSpan, Zoned, civil::DateTime};
use serde::Deserialize;
use tracing::error;

use crate::{
    RADYKO_TZ_NAME,
    radiko::{
        api::endpoint::Endpoint,
        jst_datetime::{self, RadykoDateTime},
    },
};

#[derive(Debug, Clone, Hash, PartialEq, Eq, Deserialize)]
/// 番組情報が一意になる値を返す。録音予約済み判定に利用。
pub struct ProgramId(
    StationId,
    #[serde(with = "jst_datetime")] StartAt,
    #[serde(with = "jst_datetime")] EndAt,
);

impl Display for ProgramId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", self.0, self.1, self.2)
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
                    StartAt(Self::format_datetime(start_at)?),
                    EndAt(Self::format_datetime(end_at)?),
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

    fn format_datetime(s: &str) -> anyhow::Result<Zoned> {
        DateTime::strptime(Endpoint::DATETIME_FORMAT, s)?
            .in_tz(RADYKO_TZ_NAME)
            .with_context(|| format!("format_datetime str: {s}"))
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

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct StartAt(Zoned);

impl Display for StartAt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.strftime(Endpoint::DATETIME_FORMAT))
    }
}

impl RadykoDateTime for StartAt {
    fn new(end_at: Zoned) -> Self {
        Self(end_at)
    }

    fn from_zoned(zoned: Zoned) -> Self {
        Self::new(zoned)
    }

    fn date(&self) -> Zoned {
        self.0.clone()
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct EndAt(Zoned);

impl Display for EndAt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.strftime(Endpoint::DATETIME_FORMAT))
    }
}

impl RadykoDateTime for EndAt {
    fn new(end_at: Zoned) -> Self {
        Self(end_at)
    }

    fn from_zoned(zoned: Zoned) -> Self {
        Self::new(zoned)
    }

    fn date(&self) -> Zoned {
        self.0.clone()
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct SeekStartAt(Zoned);

impl Display for SeekStartAt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.strftime(Endpoint::DATETIME_FORMAT))
    }
}

impl RadykoDateTime for SeekStartAt {
    fn new(seek_start_at: Zoned) -> Self {
        Self(seek_start_at)
    }

    fn from_zoned(zoned: Zoned) -> Self {
        Self::new(zoned)
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
        model::program::program_id::{EndAt, RadykoDateTime, SeekStartAt, StartAt},
        radiko::api::endpoint::Endpoint,
        test_helper::parse_datetime_in_tz_tokyo,
    };

    #[test]
    fn calculate_seek_start_times_test() {
        let start = parse_datetime_in_tz_tokyo("2000-01-01 00:00:00");
        let end = parse_datetime_in_tz_tokyo("2000-01-01 00:01:00");
        let mut seek_start_times =
            SeekStartAt::calculate_seek_start_times(StartAt::new(start), EndAt::new(end));
        seek_start_times.sort();

        assert_eq!(
            seek_start_times[0]
                .format(Endpoint::DATETIME_FORMAT)
                .to_string(),
            "20000101000000"
        );
        assert_eq!(
            seek_start_times[1]
                .format(Endpoint::DATETIME_FORMAT)
                .to_string(),
            "20000101000015"
        );
        assert_eq!(
            seek_start_times[2]
                .format(Endpoint::DATETIME_FORMAT)
                .to_string(),
            "20000101000030"
        );
        assert_eq!(
            seek_start_times[3]
                .format(Endpoint::DATETIME_FORMAT)
                .to_string(),
            "20000101000045".to_string()
        );

        assert_eq!(seek_start_times.len(), 4);
    }
}
