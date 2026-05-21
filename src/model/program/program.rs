use std::{path::PathBuf, time::Duration};

use crate::{
    model::program::duration_buffer::RecordingDurationBuffer,
    radiko::{dto::json::program_json::ProgramJson, jst_datetime::RadykoDateTime},
};
use futures::Stream;
use jiff::{ToSpan, Zoned, civil::DateTime};
use sanitise_file_name::sanitise;
use tracing::trace;

use crate::{
    RADYKO_TZ_NAME,
    app::{types::Seconds, utils::Utils},
    model::program::{
        error::ProgramParseError,
        program_id::{EndAt, ProgramId, StartAt, StationId},
    },
    radiko::{RadikoClient, dto::xml::program_xml::ProgramXml},
};

#[derive(Debug, Clone)]
pub struct Program {
    pub(super) program_id: ProgramId,
    pub(super) title: String,
    pub(super) performer: String,
}

impl PartialEq for Program {
    fn eq(&self, other: &Self) -> bool {
        self.program_id() == other.program_id()
    }
}

impl Program {
    pub fn new(program_id: ProgramId, title: String, performer: String) -> Self {
        Self {
            program_id,
            title,
            performer,
        }
    }

    pub fn program_id(&self) -> ProgramId {
        self.program_id.clone()
    }

    pub fn station_id(&self) -> StationId {
        self.program_id().station_id().clone()
    }

    pub fn start_at(&self) -> StartAt {
        self.program_id().start_at().clone()
    }

    pub fn end_at(&self) -> EndAt {
        self.program_id().end_at().clone()
    }

    pub fn title(&self) -> String {
        self.title.clone()
    }

    pub fn performer(&self) -> String {
        self.performer.clone()
    }

    pub fn info(&self) -> String {
        format!(
            "{}_{}_{}_{}",
            self.start_at(),
            self.station_id(),
            self.title,
            self.performer
        )
    }

    pub fn output_dir(&self, output_root_dir: PathBuf) -> PathBuf {
        output_root_dir.join(sanitise(&self.title))
    }

    pub fn output_filename(&self) -> String {
        sanitise(&format!(
            "{}_{}_{}_{}.aac",
            self.program_id().station_id().clone().get(),
            self.program_id()
                .start_at()
                .clone()
                .date()
                .strftime("%Y%m%d_%H%M%S"),
            self.title,
            self.performer,
        ))
    }

    /// 番組開始時間までの秒数を計算してsleep
    pub async fn wait_for_on_air(&self, buffer: &RecordingDurationBuffer) {
        let wait_for_on_air_secs = self.to_on_air_duration_with_buffer(None, buffer).get();
        trace!("wait for on air secs: {}", wait_for_on_air_secs);
        tokio::time::sleep(Duration::from_secs(wait_for_on_air_secs)).await;
    }

    pub fn to_on_air_duration(&self, now: Option<Zoned>) -> Seconds {
        let now = now.unwrap_or(Utils::now_in_tz_tokyo());
        Seconds::new(
            self.program_id()
                .start_at()
                .clone()
                .date()
                .duration_since(&now)
                .as_secs()
                .try_into()
                .unwrap_or(0),
        )
    }

    pub fn on_air_duration(&self) -> Seconds {
        Seconds::new(
            self.program_id()
                .end_at()
                .clone()
                .date()
                .duration_since(&self.start_at().date())
                .as_secs()
                .try_into()
                .unwrap_or(0),
        )
    }

    pub fn on_air_duration_with_buffer(&self, buffer: RecordingDurationBuffer) -> Seconds {
        // buffer分を減算して放送開始時間より前倒しの時間を計算
        let start_time = self
            .start_at()
            .date()
            .saturating_sub(buffer.start().get().try_into().unwrap_or(0).seconds());
        // buffer分を加算して放送終了時間より後の時間を計算
        let end_time = self
            .end_at()
            .date()
            .saturating_add(buffer.end().get().try_into().unwrap_or(0).seconds());

        Seconds::new(
            end_time
                .duration_since(&start_time)
                .as_secs()
                .try_into()
                .unwrap_or(0),
        )
    }

    pub fn start_time_to_program(&self) -> (StartAt, Self) {
        (self.start_at(), self.clone())
    }

    pub async fn stream_timefree_medialist_urls(
        &self,
        radiko_client: &RadikoClient,
    ) -> impl Stream<Item = anyhow::Result<String>> {
        radiko_client.stream_timefree_medialist_urls(self.program_id())
    }

    pub async fn media_list_url_for_live(
        &self,
        radiko_client: &RadikoClient,
    ) -> anyhow::Result<String> {
        Ok(radiko_client
            .media_list_url_for_live(self.station_id().clone())
            .await?
            .to_string())
    }

    fn to_on_air_duration_with_buffer(
        &self,
        now: Option<Zoned>,
        buffer: &RecordingDurationBuffer,
    ) -> Seconds {
        Seconds::new(
            self.to_on_air_duration(now)
                .get()
                .saturating_sub(buffer.start().get()),
        )
    }
}

impl TryFrom<ProgramXml> for Program {
    type Error = ProgramParseError;

    fn try_from(value: ProgramXml) -> Result<Self, Self::Error> {
        const FORMAT: &str = "%Y%m%d%H%M%S";
        let ft = DateTime::strptime(FORMAT, &value.ft)
            .map_err(|e| {
                ProgramParseError::Invalid(format!("failed parse ft: {}, error: {e:#?}", value.ft))
            })?
            .in_tz(RADYKO_TZ_NAME)
            .map_err(|e| {
                ProgramParseError::Invalid(format!(
                    "failed convert to Zoned datetime time_zone_name: {RADYKO_TZ_NAME}, error: {e:#?}"
                ))
            })?;
        let to = DateTime::strptime(FORMAT, &value.to)
            .map_err(|e| {
                ProgramParseError::Invalid(format!("failed parse to: {}, error: {e:#?}", value.to))
            })?
            .in_tz(RADYKO_TZ_NAME)
            .map_err(|e| {
                ProgramParseError::Invalid(format!(
                    "failed convert to Zoned datetime time_zone_name: {RADYKO_TZ_NAME}, error: {e:#?}"
                ))
            })?;

        Ok(Program {
            program_id: ProgramId::new(
                StationId::new(value.station_id),
                StartAt::new(ft),
                EndAt::new(to),
            ),
            title: value.title.clone(),
            performer: value.pfm.unwrap_or_default(),
        })
    }
}

impl TryFrom<ProgramJson> for Program {
    type Error = ProgramParseError;

    fn try_from(value: ProgramJson) -> Result<Self, Self::Error> {
        const FORMAT: &str = "%Y-%m-%d %H:%M:%S";
        let ft = DateTime::strptime(FORMAT, &value.start_time)
            .map_err(|e| {
                ProgramParseError::Invalid(format!("failed parse ft: {}, error: {e:#?}", value.start_time))
            })?
            .in_tz(RADYKO_TZ_NAME)
            .map_err(|e| {
                ProgramParseError::Invalid(format!(
                    "failed convert to Zoned datetime time_zone_name: {RADYKO_TZ_NAME}, error: {e:#?}"
                ))
            })?;
        let to = DateTime::strptime(FORMAT, &value.end_time)
            .map_err(|e| {
                ProgramParseError::Invalid(format!("failed parse to: {}, error: {e:#?}", value.end_time))
            })?
            .in_tz(RADYKO_TZ_NAME)
            .map_err(|e| {
                ProgramParseError::Invalid(format!(
                    "failed convert to Zoned datetime time_zone_name: {RADYKO_TZ_NAME}, error: {e:#?}"
                ))
            })?;

        Ok(Program {
            program_id: ProgramId::new(
                StationId::new(value.station_id),
                StartAt::new(ft),
                EndAt::new(to),
            ),
            title: value.title,
            performer: value.performer,
        })
    }
}
/// https://serde.rs/custom-date-format.html
pub mod jst_datetime {

    use jiff::{Zoned, civil::DateTime};
    use serde::{Deserialize, Deserializer, de::Error as _};

    use crate::RADYKO_TZ_NAME;

    const FORMAT: &str = "%Y-%m-%d %H:%M:%S";

    pub trait RadykoDateTime {
        fn new(zoned: Zoned) -> Self;

        fn from_zoned(zoned: Zoned) -> Self;

        fn date(&self) -> Zoned;

        fn format(&self, format: &str) -> String {
            self.date().strftime(format).to_string()
        }
    }

    // The signature of a deserialize_with function must follow the pattern:
    //
    //    fn deserialize<'de, D>(D) -> Result<T, D::Error>
    //    where
    //        D: Deserializer<'de>
    //
    // although it may also be generic over the output types T.
    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        T: RadykoDateTime,
        D: Deserializer<'de>,
    {
        let s =
            String::deserialize(deserializer).map_err(|e| D::Error::custom(format!("{e:#?}")))?;
        let datetime =
            DateTime::strptime(FORMAT, &s).map_err(|e| D::Error::custom(format!("{e:#?}")))?;
        datetime
            .in_tz(RADYKO_TZ_NAME)
            .map(T::from_zoned)
            .map_err(|e| {
                D::Error::custom(format!(
                    "jst_datetime deserialize error s: {s} error: {e:#?}"
                ))
            })
    }
}

/// https://serde.rs/custom-date-format.html
pub mod program_id {

    use serde::{Deserialize, Deserializer, de::Error as _};

    use crate::model::program::program_id::ProgramId;

    // The signature of a deserialize_with function must follow the pattern:
    //
    //    fn deserialize<'de, D>(D) -> Result<T, D::Error>
    //    where
    //        D: Deserializer<'de>
    //
    // although it may also be generic over the output types T.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<ProgramId, D::Error>
    where
        D: Deserializer<'de>,
    {
        ProgramId::deserialize(deserializer)
            .map_err(|e| D::Error::custom(format!("program_id deserialize error: {e:#?}")))
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
        let dummy_title = "オールナイトニッポン".to_string();
        let dummy_performer = "フワちゃん".to_string();
        let program = Program::new(
            ProgramId::new(
                StationId::new("LFR".to_string()),
                StartAt::new(dummy_start_time),
                EndAt::new(dummy_end_time),
            ),
            dummy_title,
            dummy_performer,
        );

        assert_eq!(
            on_air_duration.total(Unit::Second).unwrap() as u64,
            program.on_air_duration().get()
        );
    }

    #[test]
    fn on_air_duration_with_buffer_test() {
        let on_air_duration = 1.hours();
        let dummy_start_time = parse_datetime_in_tz_tokyo("2000-01-01 00:00:00");
        let dummy_end_time = dummy_start_time.checked_add(on_air_duration).unwrap();

        let start_buffer_seconds = 1.minutes().get_seconds() as u64;
        let end_buffer_seconds = 1.minutes().get_seconds() as u64;
        let dummy_title = "オールナイトニッポン".to_string();
        let dummy_performer = "フワちゃん".to_string();
        let program = Program::new(
            ProgramId::new(
                StationId::new("LFR".to_string()),
                StartAt::new(dummy_start_time),
                EndAt::new(dummy_end_time),
            ),
            dummy_title,
            dummy_performer,
        );

        assert_eq!(
            on_air_duration.total(Unit::Second).unwrap() as u64
                + start_buffer_seconds
                + end_buffer_seconds,
            program
                .on_air_duration_with_buffer(RecordingDurationBuffer::new(
                    Seconds::new(start_buffer_seconds),
                    Seconds::new(end_buffer_seconds)
                ))
                .get()
        );
    }
}
