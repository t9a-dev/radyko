mod common;

#[cfg(test)]
mod recording_program_test {
    use std::{path::PathBuf, sync::Arc, time::Duration};

    use crate::common::tests_common::TestProgram;
    use chrono::{TimeDelta, Utc};
    use chrono_tz::Asia::Tokyo;
    use radyko::{
        app::{
            config::{RecordingConfig, RecordingDurationBufferConfig},
            program_reserver::ProgramReserver,
        },
        telemetry::init_telemetry,
    };
    use tempfile::TempDir;

    use crate::common::tests_common::{TEST_AREA_ID, radiko_client};

    #[tokio::test]
    #[ignore = "実際に録音処理を走らせる都合上数秒を要するため"]
    /// cargo test -- --ignored で実行
    async fn now_on_air_recording_test() -> anyhow::Result<()> {
        init_telemetry("now_on_air_recording_test", None);
        let radiko_client = radiko_client().await;
        let now_on_air_programs = radiko_client
            .now_on_air_programs(Some(TEST_AREA_ID))
            .await?;
        let temp_dir = TempDir::new()?;
        let recording_config = RecordingConfig {
            output_dir: PathBuf::from(temp_dir.path()),
            schedule_update_interval_secs: 60,
            duration_buffer_secs: Some(RecordingDurationBufferConfig { start: 0, end: 0 }),
        };
        let program_reserver = Arc::new(ProgramReserver::new(
            radiko_client.clone(),
            recording_config,
        ));
        let now = Utc::now().with_timezone(&Tokyo);
        let recording_duration_secs = 5;
        // 今放送している適当な番組を録音
        let test_reserve_program = now_on_air_programs[0].with_start_end_time(
            now,
            now.checked_add_signed(TimeDelta::seconds(recording_duration_secs))
                .unwrap(),
        );
        program_reserver.reserve(test_reserve_program).await?;

        // バックグラウンドで録音処理が実行される時間待機
        // 録音処理でエラーが発生しないことのみを検証
        tokio::time::sleep(Duration::from_secs(recording_duration_secs as u64)).await;

        Ok(())
    }
}
