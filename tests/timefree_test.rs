mod common;

#[cfg(test)]
mod timefree_test {
    use std::{ops::Not, path::PathBuf, str::FromStr, sync::Arc};

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
        let radiko_client = radiko_client().await;
        let timefree_programs = radiko_client
            .search_timefree_programs_with_keyword(
                "オールナイトニッポン".to_string(),
                Some("LFR"),
                None,
            )
            .await?
            .to_vec();
        let dummy_program = timefree_programs.first().unwrap();

        println!("resolve keyword program: {:#?}", dummy_program);
        /*
           station_id: "LFR"
           start_time: 2026-04-26T03:00:00JST,
           end_time: 2026-04-26T05:00:00JST,
        */

        let output_root_dir = PathBuf::from_str("./timefree_test")?;
        dummy_program
            .download_timefree(
                output_root_dir,
                Arc::clone(radiko_client),
                reqwest::Client::new(),
            )
            .await?;

        Ok(())
    }
}
