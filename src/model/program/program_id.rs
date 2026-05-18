use std::fmt::{self, Display};

use anyhow::{Context, bail};
use jiff::{Zoned, civil::DateTime};

use crate::{RADYKO_TZ_NAME, radiko::api::endpoint::Endpoint};

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
