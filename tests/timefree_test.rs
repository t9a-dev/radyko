mod common;

#[cfg(test)]
mod timefree_test {
    use std::{ops::Not, path::Path, sync::Arc};

    use radyko::{
        application::{state::RecorderState, usecase::download_timefree::DownloadTimeFreeUseCase},
        infrastructure::new_file_reserved_repository,
    };
    use reqwest::Client;
    use tempfile::NamedTempFile;

    use crate::common::tests_common::{exists_file, radiko_client};

    #[tokio::test]
    #[ignore = "radiko apiに依存"]
    async fn timefree_search_test() -> anyhow::Result<()> {
        let radiko_client = radiko_client().await;
        let programs = radiko_client
            .search_timefree_programs_with_keyword(
                "オールナイトニッポン".to_string(),
                Some("LFR"),
                None,
            )
            .await?;

        assert!(programs.to_vec().is_empty().not());
        println!("resolve keyword programs: {:#?}", programs);

        /*
           station_id: "LFR"
           start_time: 2026-04-26T03:00:00JST,
           end_time: 2026-04-26T05:00:00JST,
        */

        Ok(())
    }

    #[tokio::test]
    #[ignore = "radiko apiに依存"]
    async fn download_timefree_test() -> anyhow::Result<()> {
        // Arrange
        let radiko_client = radiko_client().await;
        let timefree_programs = radiko_client
            .search_timefree_programs_with_keyword(
                "オールナイトニッポン".to_string(),
                Some("LFR"),
                None,
            )
            .await?
            .to_vec();
        let first_program = timefree_programs.first().unwrap();
        println!("resolve keyword program: {:#?}", first_program);

        let output_root_dir = Path::new("./timefree_test");
        let _ = std::fs::create_dir(output_root_dir);
        let reserved_state_file_path = NamedTempFile::new_in(output_root_dir)?;
        let recorder_state = RecorderState::new(new_file_reserved_repository(
            reserved_state_file_path.path().to_path_buf(),
        ));
        recorder_state.add_reserve_programs(vec![first_program.clone()]);
        let sut = DownloadTimeFreeUseCase::new(
            Client::new(),
            Arc::clone(radiko_client),
            Arc::new(recorder_state),
        );

        // Act
        sut.exec(output_root_dir).await?;

        // Assert
        assert!(exists_file(
            output_root_dir.to_str().unwrap(),
            &first_program.output_filename().to_string()
        ));
        Ok(())
    }
}
