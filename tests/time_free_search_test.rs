mod common;

#[cfg(test)]
mod time_free_search_test {
    use std::ops::Not;

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
}
