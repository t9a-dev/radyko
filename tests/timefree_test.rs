mod common;

#[cfg(test)]
mod timefree_test {
    use std::{ops::Not, path::PathBuf, str::FromStr};

    use radyko::app::{hls::StreamHandler, program_reserver::ReserveProgram};

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
    async fn collect_timefree_medialist_urls_test() -> anyhow::Result<()> {
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

        let medialist_urls = radiko_client
            .collect_timefree_medialist_urls(
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

        let medialist_urls = radiko_client
            .collect_timefree_medialist_urls(
                dummy_program.station_id.to_string(),
                dummy_program.start_time,
                dummy_program.end_time,
            )
            .await?;
        assert!(medialist_urls.is_empty().not());

        println!("medialist urls: {:#?}", medialist_urls);

        let stream_handler = StreamHandler::new(reqwest::Client::new());

        let output_dir_path = PathBuf::from_str("./timefree_test")?;
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
