mod common;

#[cfg(test)]
mod resolve_programs_test {
    use std::ops::Not;

    use crate::common::tests_common::{load_example_config, radiko_client};
    use radyko::{app::program_selector::ProgramSelector, model::program::programs::Programs};

    #[tokio::test]
    #[ignore = "radiko apiに依存"]
    async fn resolve_keyword_programs() -> anyhow::Result<()> {
        let radiko_client = radiko_client().await;
        let mut radyko_config = load_example_config()?;
        {
            // "オールナイトニッポン"をキーワードに加えて検索結果が常に1件以上になるように調整
            let keywords = &mut radyko_config.keywords.as_mut().unwrap();
            keywords.0.insert(
                radyko::app::types::Station::Id("LFR".to_string()),
                vec!["オールナイトニッポン".to_string()],
            );
        }
        let program_selectors = ProgramSelector::from_keywords(radyko_config.keywords.unwrap());
        let result = Programs::resolve_selectors(radiko_client, program_selectors).await?;

        assert!(result.is_empty().not());
        println!("resolve keyword programs: {:#?}", result);

        Ok(())
    }

    #[tokio::test]
    #[ignore = "radiko apiに依存"]
    async fn resolve_rule_programs() -> anyhow::Result<()> {
        let radyko_config = load_example_config()?;
        let radiko_client = radiko_client().await;
        let program_selectors = ProgramSelector::from_rules(radyko_config.rules.unwrap())?;
        let result = Programs::resolve_selectors(radiko_client, program_selectors).await?;

        assert!(result.is_empty().not());
        println!("resolve rule programs: {:#?}", result);

        Ok(())
    }
}
