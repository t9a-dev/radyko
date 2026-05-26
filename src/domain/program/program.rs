use std::{fs, path::PathBuf, sync::Arc, time::Duration};

use crate::{
    application::{
        hls::{ByteSize, StreamHandler},
        ports::RadikoClient,
    },
    domain::program::{
        BufferSecs, RadykoDateTime, RecordingDurationBuffers, duration_buffer::StartBuffer,
    },
};
use futures::stream::BoxStream;
use jiff::{ToSpan, Zoned};
use sanitise_file_name::sanitise;
use tracing::trace;

use crate::{
    application::utils::Utils,
    domain::program::{EndAt, ProgramId, StartAt, StationId},
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
            self.start_at().display(),
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
    pub async fn wait_for_live_on_air(&self, start_buffer: &StartBuffer) {
        let wait_for_on_air_secs = self.to_live_on_air_duration(None, start_buffer).as_secs();
        trace!("wait for on air secs: {}", wait_for_on_air_secs);
        tokio::time::sleep(Duration::from_secs(wait_for_on_air_secs)).await;
    }

    pub fn on_air_duration_for_timefree(&self) -> Duration {
        Duration::from_secs(
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

    pub fn on_air_duration_for_live(&self, buffers: &RecordingDurationBuffers) -> Duration {
        let start_time = self.start_at().date().saturating_sub(
            buffers
                .start_buffer()
                .secs()
                .try_into()
                .unwrap_or(0)
                .seconds(),
        );
        let end_time = self.end_at().date().saturating_add(
            buffers
                .end_buffer()
                .secs()
                .try_into()
                .unwrap_or(0)
                .seconds(),
        );

        Duration::from_secs(
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
        radiko_client: Arc<dyn RadikoClient>,
    ) -> BoxStream<'static, anyhow::Result<String>> {
        radiko_client.stream_timefree_medialist_urls(self.program_id())
    }

    pub async fn media_list_url_for_live(
        &self,
        radiko_client: Arc<dyn RadikoClient>,
    ) -> anyhow::Result<String> {
        Ok(radiko_client
            .media_list_url_for_live(self.station_id().clone())
            .await?
            .to_string())
    }

    pub async fn download_timefree(
        &self,
        output_root_dir: PathBuf,
        radiko_client: Arc<dyn RadikoClient>,
        http_client: reqwest::Client,
    ) -> anyhow::Result<()> {
        let stream_media_list_urls = self.stream_timefree_medialist_urls(radiko_client).await;
        let recorded_file_path = StreamHandler::new(http_client)
            .download_timefree_program(
                stream_media_list_urls,
                self.output_dir(output_root_dir),
                &self.output_filename(),
            )
            .await?;
        let recorded_file = fs::File::open(recorded_file_path)?;
        StreamHandler::verify_recorded_file(
            ByteSize::from_bytes(recorded_file.metadata()?.len()),
            self.on_air_duration_for_timefree(),
        )?;

        Ok(())
    }

    fn to_live_on_air_duration(&self, now: Option<Zoned>, start_buffer: &StartBuffer) -> Duration {
        Duration::from_secs(
            self.program_id()
                .start_at()
                .clone()
                .date()
                // .saturating_sub(start_buffer.duration())
                .duration_since(&now.unwrap_or(Utils::now_in_tz_tokyo()))
                .as_secs()
                .try_into()
                .unwrap_or(0_u64)
                .saturating_sub(start_buffer.secs()),
        )
    }
}

/// https://serde.rs/custom-date-format.html
// pub mod program_id {

//     use serde::{Deserialize, Deserializer, de::Error as _};

//     use crate::domain::program::ProgramId;

//     // The signature of a deserialize_with function must follow the pattern:
//     //
//     //    fn deserialize<'de, D>(D) -> Result<T, D::Error>
//     //    where
//     //        D: Deserializer<'de>
//     //
//     // although it may also be generic over the output types T.
//     #[allow(dead_code)]
//     pub fn deserialize<'de, D>(deserializer: D) -> Result<ProgramId, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         ProgramId::deserialize(deserializer)
//             .map_err(|e| D::Error::custom(format!("program_id deserialize error: {e:#?}")))
//     }
// }

#[cfg(test)]
mod tests {

    use crate::{
        domain::program::{EndBuffer, StartBuffer},
        test_helper::parse_datetime_in_tz_tokyo,
    };

    use super::*;

    #[test]
    fn to_live_on_air_duration_test() -> anyhow::Result<()> {
        let on_air_duration = Duration::from_hours(1);
        let dummy_start_time = parse_datetime_in_tz_tokyo("2000-01-15 00:00:00");
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

        // 番組開始時間までは1分の猶予がある
        let now = parse_datetime_in_tz_tokyo("2000-01-14 23:59:00");
        // 番組開始時間より30秒早く録音開始したい
        let to_live_on_air_duration =
            program.to_live_on_air_duration(Some(now), &StartBuffer::new(Duration::from_secs(30)));

        // bufferを含めると録音開始時間までの猶予は30秒
        assert_eq!(to_live_on_air_duration, Duration::from_secs(30));

        Ok(())
    }

    #[test]
    fn on_air_duration_for_timefree_test() {
        let on_air_duration = Duration::from_hours(1);
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
            program.on_air_duration_for_timefree(),
            Duration::from_hours(1)
        );
    }

    #[test]
    fn on_air_duration_for_live_test() {
        let on_air_duration = Duration::from_hours(1);
        let dummy_start_time = parse_datetime_in_tz_tokyo("2000-01-01 00:00:00");
        let dummy_end_time = dummy_start_time.checked_add(on_air_duration).unwrap();

        let start_buffer = StartBuffer::new(Duration::from_secs(30));
        let end_buffer = EndBuffer::new(Duration::from_secs(90));
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
        let recording_duration_buffers = RecordingDurationBuffers::new(start_buffer, end_buffer);

        assert_eq!(
            program.on_air_duration_for_live(&recording_duration_buffers),
            on_air_duration
                .saturating_add(start_buffer.duration())
                .saturating_add(end_buffer.duration())
        );
    }
}
