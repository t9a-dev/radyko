use chrono::{DateTime, TimeDelta, TimeZone, Utc};
use chrono_tz::{Asia::Tokyo, Tz};
use serde_derive::{Deserialize, Serialize};

use crate::radiko::jst_datetime;

#[derive(Debug, Copy, Clone)]
pub struct Seconds(pub u64);

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
/// 番組情報が一意になる値を返す。録音予約済み判定に利用。
pub struct ProgramId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Programs {
    pub data: Vec<Program>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Program {
    #[serde(with = "jst_datetime")]
    pub start_time: DateTime<Tz>,
    #[serde(with = "jst_datetime")]
    pub end_time: DateTime<Tz>,
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
            start_time: Tokyo
                .from_local_datetime(&DateTime::UNIX_EPOCH.naive_local())
                .unwrap(),
            end_time: Tokyo
                .from_local_datetime(&DateTime::UNIX_EPOCH.naive_local())
                .unwrap(),
            station_id: Default::default(),
            title: Default::default(),
            performer: Default::default(),
            // start_time_s: Default::default(),
            // end_time_s: Default::default(),
            // info: Default::default(),
            // description: Default::default(),
            // img: Default::default(),
        }
    }
}

impl Program {
    pub fn new(start_time: DateTime<Tz>, end_time: DateTime<Tz>) -> Self {
        Self {
            start_time,
            end_time,
            ..Default::default()
        }
    }

    pub fn program_id(&self) -> ProgramId {
        ProgramId(format!("{}_{}", self.station_id, self.start_time))
    }

    pub fn get_info(&self) -> String {
        format!(
            "{}_{}_{}_{}",
            self.start_time, self.station_id, self.title, self.performer
        )
    }

    pub fn to_on_air_duration(&self, now: Option<DateTime<Tz>>) -> Seconds {
        let now = now.unwrap_or(Utc::now().with_timezone(&Tokyo));
        Seconds(
            self.start_time
                .signed_duration_since(now)
                .num_seconds()
                .try_into()
                .unwrap_or(0),
        )
    }

    pub fn on_air_duration(&self) -> Seconds {
        Seconds(
            self.end_time
                .signed_duration_since(self.start_time)
                .num_seconds()
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
            .checked_sub_signed(TimeDelta::seconds(start_buffer.try_into().unwrap_or(0)))
            .unwrap();
        // buffer分を加算して放送終了時間より後の時間を計算
        let end_time = self
            .end_time
            .checked_add_signed(TimeDelta::seconds(end_buffer.try_into().unwrap_or(0)))
            .unwrap();

        Seconds(
            end_time
                .signed_duration_since(start_time)
                .num_seconds()
                .try_into()
                .unwrap_or(0),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDateTime;

    #[test]
    fn on_air_duration_test() {
        let on_air_duration = TimeDelta::hours(1);
        let dummy_start_time = Tokyo
            .from_local_datetime(
                &NaiveDateTime::parse_from_str("2000-01-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap(),
            )
            .unwrap();
        let dummy_end_time = dummy_start_time
            .checked_add_signed(on_air_duration)
            .unwrap();

        let program = Program::new(dummy_start_time, dummy_end_time);
        assert_eq!(
            on_air_duration.num_seconds() as u64,
            program.on_air_duration().0
        );
    }

    #[test]
    fn on_air_duration_with_buffer_test() {
        let on_air_duration = TimeDelta::hours(1);
        let dummy_start_time = Tokyo
            .from_local_datetime(
                &NaiveDateTime::parse_from_str("2000-01-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap(),
            )
            .unwrap();
        let dummy_end_time = dummy_start_time
            .checked_add_signed(on_air_duration)
            .unwrap();

        let one_minutes_seconds = TimeDelta::minutes(1).num_seconds() as u64;
        let program = Program::new(dummy_start_time, dummy_end_time);
        assert_eq!(
            on_air_duration.num_seconds() as u64 + one_minutes_seconds * 2,
            program
                .on_air_duration_with_buffer(
                    Seconds(one_minutes_seconds),
                    Seconds(one_minutes_seconds)
                )
                .0
        );
    }
}
