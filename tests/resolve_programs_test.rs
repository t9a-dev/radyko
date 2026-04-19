mod common;

#[cfg(test)]
mod resolve_programs_test {
    use std::ops::Not;

    use crate::common::tests_common::{load_example_config, radiko_client};
    use radyko::app::{program_resolver::resolve_selector, program_selector::ProgramSelector};

    #[tokio::test]
    #[ignore = "radiko apiに依存"]
    async fn resolve_keyword_programs() -> anyhow::Result<()> {
        let radyko_config = load_example_config()?;
        let radiko_client = radiko_client().await;

        let mut result = Vec::new();
        let program_selectors = ProgramSelector::from_keywords(radyko_config.keywords.unwrap());
        for program_selector in program_selectors {
            let programs = resolve_selector(&radiko_client, program_selector).await?;
            result.extend(programs);
        }

        assert!(result.is_empty().not());
        println!("resolve keyword programs: {:#?}", result);

        Ok(())
    }

    #[tokio::test]
    #[ignore = "radiko apiに依存"]
    async fn resolve_rule_programs() -> anyhow::Result<()> {
        let radyko_config = load_example_config()?;
        let radiko_client = radiko_client().await;

        let mut result = Vec::new();
        let program_selectors = ProgramSelector::from_rules(radyko_config.rules.unwrap())?;
        for program_selector in program_selectors {
            let programs = resolve_selector(&radiko_client, program_selector).await?;
            result.extend(programs);
        }

        assert!(result.is_empty().not());
        println!("resolve rule programs: {:#?}", result);

        Ok(())
    }
}
