mod common;

#[cfg(test)]
mod resolve_programs_test {
    use std::{collections::HashMap, ops::Not, sync::Arc};

    use crate::common::tests_common::{TEST_STATION_ID, radiko_client};
    use radyko::{
        app::{
            config::{RadykoConfigKeywords, RadykoConfigRules},
            types::Station,
        },
        domain::program::Programs,
    };

    #[tokio::test]
    #[ignore = "radiko apiに依存"]
    async fn resolve_keyword_programs() -> anyhow::Result<()> {
        let radiko_client = radiko_client().await;
        // "オールナイトニッポン"をキーワードに加えて検索結果が常に1件以上になるように調整
        let mut keywords = HashMap::new();
        keywords.insert(
            radyko::app::types::Station::Id("LFR".to_string()),
            vec!["オールナイトニッポン".to_string()],
        );
        let program_selectors = RadykoConfigKeywords::new(keywords).into_program_selectors();
        let result =
            Programs::resolve_selectors(Arc::clone(radiko_client), program_selectors).await?;

        assert!(result.is_empty().not());
        println!("resolve keyword programs: {:#?}", result);

        Ok(())
    }

    #[tokio::test]
    #[ignore = "radiko apiに依存"]
    async fn resolve_rule_programs() -> anyhow::Result<()> {
        let radiko_client = radiko_client().await;
        let rules: HashMap<Station, Vec<String>> = HashMap::from_iter(vec![(
            Station::Id(TEST_STATION_ID.to_string()),
            vec!["* * * * * *".to_string()],
        )]);
        let program_selectors = RadykoConfigRules::new(rules).try_into_program_selectors(None)?;
        let result =
            Programs::resolve_selectors(Arc::clone(radiko_client), program_selectors).await?;

        assert!(result.is_empty().not());
        println!("resolve rule programs: {:#?}", result);

        Ok(())
    }
}
