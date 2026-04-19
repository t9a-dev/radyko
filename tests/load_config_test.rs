mod common;

#[cfg(test)]
mod load_config_test {

    use radyko::app::config::RadykoConfig;
    use std::{fs::File, io::BufReader};

    use crate::common::tests_common::{
        TEST_EMPTY_KEYWORDS_CONFIG_PATH, TEST_EMPTY_RULES_CONFIG_PATH, load_example_config,
    };

    #[test]
    fn valid_parse_test() -> anyhow::Result<()> {
        let radyko_config = load_example_config()?;

        let mut insta_settings = insta::Settings::clone_current();
        insta_settings.set_sort_maps(true);
        insta_settings.bind(|| insta::assert_yaml_snapshot!(radyko_config));

        Ok(())
    }

    #[test]
    fn valid_empty_keywords_parse_test() -> anyhow::Result<()> {
        let config = File::open(TEST_EMPTY_KEYWORDS_CONFIG_PATH)?;
        let reader = BufReader::new(config);
        let radyko_config = RadykoConfig::parse(reader)?;

        let mut insta_settings = insta::Settings::clone_current();
        insta_settings.set_sort_maps(true);
        insta_settings.bind(|| insta::assert_yaml_snapshot!(radyko_config));

        Ok(())
    }

    #[test]
    fn valid_empty_rules_parse_test() -> anyhow::Result<()> {
        let config = File::open(TEST_EMPTY_RULES_CONFIG_PATH)?;
        let reader = BufReader::new(config);
        let radyko_config = RadykoConfig::parse(reader)?;

        let mut insta_settings = insta::Settings::clone_current();
        insta_settings.set_sort_maps(true);
        insta_settings.bind(|| insta::assert_yaml_snapshot!(radyko_config));

        Ok(())
    }
}
