use std::{path::PathBuf, time::Duration};

use crate::{model::program::duration_buffer::RecordingDurationBuffer, radiko::jst_datetime};
use futures::Stream;
use jiff::{ToSpan, Zoned, civil::DateTime};
use sanitise_file_name::sanitise;
use serde_derive::{Deserialize, Serialize};
use tracing::trace;

use crate::{
    RADYKO_TZ_NAME,
    app::{types::Seconds, utils::Utils},
    model::program::{
        error::ProgramParseError,
        program_id::{EndAt, ProgramId, StartAt, StationId},
    },
    radiko::{RadikoClient, dto::program_xml::ProgramXml},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Program {
    #[serde(with = "jst_datetime")]
    pub(super) start_time: Zoned,
    #[serde(with = "jst_datetime")]
    pub(super) end_time: Zoned,
    pub(super) station_id: String,
    pub(super) title: String,
    pub(super) performer: String,
}

impl Default for Program {
    fn default() -> Self {
        Self {
            start_time: Utils::now_in_tz_tokyo(),
            end_time: Utils::now_in_tz_tokyo(),
            station_id: "TEST".to_string(),
            title: "テスト番組名".to_string(),
            performer: "テスト出演者".to_string(),
        }
    }
}

impl PartialEq for Program {
    fn eq(&self, other: &Self) -> bool {
        self.start_time == other.start_time
            && self.end_time == other.end_time
            && self.station_id == other.station_id
    }
}

impl Program {
    pub fn new(station_id: String, start_time: Zoned, end_time: Zoned) -> Self {
        Self {
            station_id,
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

    pub fn info(&self) -> String {
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

    /// 番組開始時間までの秒数を計算してsleep
    pub async fn wait_for_on_air(&self, buffer: &RecordingDurationBuffer) {
        let wait_for_on_air_secs = self.to_on_air_duration_with_buffer(None, buffer).0;
        trace!("wait for on air secs: {}", wait_for_on_air_secs);
        tokio::time::sleep(Duration::from_secs(wait_for_on_air_secs)).await;
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

    pub fn on_air_duration_with_buffer(&self, buffer: RecordingDurationBuffer) -> Seconds {
        let RecordingDurationBuffer {
            start: Seconds(start_buffer),
            end: Seconds(end_buffer),
        } = buffer;

        // buffer分を減算して放送開始時間より前倒しの時間を計算
        let start_time = self
            .start_time
            .checked_sub(start_buffer.try_into().unwrap_or(0).seconds())
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

    pub fn start_time_to_program(&self) -> (Zoned, Self) {
        (self.start_time.clone(), self.clone())
    }

    pub async fn stream_timefree_medialist_urls(
        &self,
        radiko_client: &RadikoClient,
    ) -> impl Stream<Item = anyhow::Result<String>> {
        radiko_client.stream_timefree_medialist_urls(
            self.station_id.clone(),
            self.start_time.clone(),
            self.end_time.clone(),
        )
    }

    pub async fn media_list_url_for_live(
        &self,
        radiko_client: &RadikoClient,
    ) -> anyhow::Result<String> {
        Ok(radiko_client
            .media_list_url_for_live(&self.station_id)
            .await?
            .to_string())
    }

    fn to_on_air_duration_with_buffer(
        &self,
        now: Option<Zoned>,
        buffer: &RecordingDurationBuffer,
    ) -> Seconds {
        Seconds(
            self.to_on_air_duration(now)
                .0
                .saturating_sub(buffer.start.0),
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
            start_time: ft,
            end_time: to,
            station_id: value.station_id,
            title: value.title.clone(),
            performer: value.pfm.unwrap_or_default(),
        })
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

        let program = Program::new("LFR".to_string(), dummy_start_time, dummy_end_time);
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
        let program = Program::new("LFR".to_string(), dummy_start_time, dummy_end_time);
        assert_eq!(
            on_air_duration.total(Unit::Second).unwrap() as u64
                + start_buffer_seconds
                + end_buffer_seconds,
            program
                .on_air_duration_with_buffer(RecordingDurationBuffer::new(
                    Seconds(start_buffer_seconds),
                    Seconds(end_buffer_seconds)
                ))
                .0
        );
    }
}
