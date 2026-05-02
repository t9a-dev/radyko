mod common;

#[cfg(test)]
mod time_free_test {
    use std::{ops::Not, path::PathBuf, str::FromStr};

    use radyko::app::{hls::StreamHandler, program_reserver::ReserveProgram};

    use crate::common::tests_common::radiko_client;

    #[tokio::test]
    #[ignore = "radiko apiに依存"]
    async fn time_free_search_test() -> anyhow::Result<()> {
        let radiko_client = radiko_client().await;
        let programs = radiko_client
            .search_time_free_programs_with_keyword(
                "オールナイトニッポン".to_string(),
                Some("LFR"),
                None,
            )
            .await?;

        assert!(programs.data.is_empty().not());
        println!("resolve keyword programs: {:#?}", programs);

        /*
           start_time: 2026-04-26T03:00:00JST,
           end_time: 2026-04-26T05:00:00JST,
           station_id: "LFR"
        */

        Ok(())
    }

    #[tokio::test]
    #[ignore = "radiko apiに依存"]
    async fn collect_time_free_medialist_urls_test() -> anyhow::Result<()> {
        let radiko_client = radiko_client().await;
        let time_free_programs = radiko_client
            .search_time_free_programs_with_keyword(
                "オールナイトニッポン".to_string(),
                Some("LFR"),
                None,
            )
            .await?;
        let dummy_program = time_free_programs.data.first().unwrap();

        println!("resolve keyword program: {:#?}", dummy_program);
        /*
           start_time: 2026-04-26T03:00:00JST,
           end_time: 2026-04-26T05:00:00JST,
           station_id: "LFR"
        */

        let medialist_urls = radiko_client
            .collect_time_free_medialist_urls(
                dummy_program.station_id.to_string(),
                dummy_program.start_time,
                dummy_program.end_time,
            )
            .await?;
        assert!(medialist_urls.is_empty().not());

        println!(
            "medialist urls: {:#?}, count: {}",
            medialist_urls,
            medialist_urls.len(),
        );

        Ok(())
    }

    #[tokio::test]
    #[ignore = "radiko apiに依存"]
    async fn download_time_free_test() -> anyhow::Result<()> {
        let radiko_client = radiko_client().await;
        let time_free_programs = radiko_client
            .search_time_free_programs_with_keyword(
                "オールナイトニッポン".to_string(),
                Some("LFR"),
                None,
            )
            .await?;
        let dummy_program = time_free_programs.data.first().unwrap();

        println!("resolve keyword program: {:#?}", dummy_program);
        /*
           start_time: 2026-04-26T03:00:00JST,
           end_time: 2026-04-26T05:00:00JST,
           station_id: "LFR"
        */

        let medialist_urls = radiko_client
            .collect_time_free_medialist_urls(
                dummy_program.station_id.to_string(),
                dummy_program.start_time,
                dummy_program.end_time,
            )
            .await?;
        assert!(medialist_urls.is_empty().not());

        println!("medialist urls: {:#?}", medialist_urls);

        let stream_handler = StreamHandler::new(reqwest::Client::new());

        let output_dir_path = PathBuf::from_str("./time_free_test")?;
        let _ = std::fs::create_dir_all(&output_dir_path)?;
        let download_program = ReserveProgram::new(dummy_program.clone(), output_dir_path, None);
        let _ = stream_handler
            .download_timefree_program(
                medialist_urls,
                download_program.output_dir(),
                &download_program.output_filename(),
            )
            .await?;

        Ok(())
    }
}
