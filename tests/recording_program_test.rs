mod common;

#[cfg(test)]
mod recording_program_test {
    use std::{sync::Arc, time::Duration};

    use radyko::{
        application::{
            state::RecorderState, usecase::reserve_program::ReserveProgramUseCase, utils::Utils,
        },
        domain::program::{
            BufferSecs, EndAt, EndBuffer, Program, ProgramId, RadykoDateTime,
            RecordingDurationBuffers, StartAt, StartBuffer,
        },
        infrastructure::new_file_reserved_repository,
        telemetry::init_telemetry,
    };
    use tempfile::{NamedTempFile, TempDir};

    use crate::common::tests_common::{TEST_AREA_ID, exists_file, radiko_client};

    #[tokio::test]
    #[ignore = "実際に録音処理を走らせる都合上数秒を要するため"]
    /// cargo test -- --ignored で実行
    async fn now_on_air_recording_test() -> anyhow::Result<()> {
        // Arrange
        init_telemetry("now_on_air_recording_test", None);
        let radiko_client = radiko_client().await;
        let now_on_air_programs = radiko_client
            .now_on_air_programs(Some(TEST_AREA_ID))
            .await?;
        let now = Utils::now_in_tz_tokyo();
        let recording_duration_secs = 5;
        // 今放送している適当な番組を録音
        let program = now_on_air_programs.first().unwrap();
        let test_reserve_program = Program::new(
            ProgramId::new(
                program.station_id(),
                StartAt::new(now.clone()),
                EndAt::new(now.saturating_add(Duration::from_secs(recording_duration_secs))),
            ),
            program.title(),
            program.performer(),
        );
        let recorder_state = RecorderState::new(new_file_reserved_repository(
            NamedTempFile::new_in("./")?.path().to_path_buf(),
        ));
        let sut = ReserveProgramUseCase::new(Arc::clone(radiko_client), Arc::new(recorder_state));
        let output_root_dir = TempDir::new_in("./")?;

        // Act
        sut.exec(
            vec![test_reserve_program.clone()],
            output_root_dir.path().to_path_buf(),
            RecordingDurationBuffers::new(
                StartBuffer::new(Duration::from_secs(0)),
                EndBuffer::new(Duration::from_secs(0)),
            ),
        )
        .await?;
        // バックグラウンドで録音処理が実行される時間待機
        tokio::time::sleep(Duration::from_secs(recording_duration_secs)).await;

        // Assert
        assert!(exists_file(
            output_root_dir.path().to_str().unwrap(),
            &test_reserve_program.output_filename().to_string()
        ));
        Ok(())
    }
}
