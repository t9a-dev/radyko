mod common;

#[cfg(test)]
mod timefree_test {
    use std::{fs, ops::Not, path::PathBuf, str::FromStr, time::Duration};

    use futures::pin_mut;
    use radyko::app::{
        hls::{ByteSize, StreamHandler},
        program_reserver::ReserveProgram,
    };

    use crate::common::tests_common::radiko_client;

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

        assert!(programs.data.is_empty().not());
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
        let radiko_client = radiko_client().await;
        let timefree_programs = radiko_client
            .search_timefree_programs_with_keyword(
                "オールナイトニッポン".to_string(),
                Some("LFR"),
                None,
            )
            .await?;
        let dummy_program = timefree_programs.data.first().unwrap();

        println!("resolve keyword program: {:#?}", dummy_program);
        /*
           station_id: "LFR"
           start_time: 2026-04-26T03:00:00JST,
           end_time: 2026-04-26T05:00:00JST,
        */

        let stream_medialist_urls = radiko_client.stream_timefree_medialist_urls(
            dummy_program.station_id.to_string(),
            dummy_program.start_time,
            dummy_program.end_time,
        );
        pin_mut!(stream_medialist_urls);

        let stream_handler = StreamHandler::new(reqwest::Client::new());
        let output_dir_path = PathBuf::from_str("./timefree_test")?;
        let _ = std::fs::create_dir_all(&output_dir_path)?;
        let download_program = ReserveProgram::new(dummy_program.clone(), output_dir_path, None);
        let downloaded_file_path = stream_handler
            .download_timefree_program(
                stream_medialist_urls,
                download_program.output_dir(),
                &download_program.output_filename(),
            )
            .await?;
        let file = fs::File::open(downloaded_file_path)?;
        StreamHandler::verify_recorded_file(
            ByteSize::from_bytes(file.metadata()?.len()),
            Duration::from_secs(download_program.on_air_duration().0),
        )?;

        Ok(())
    }
}
