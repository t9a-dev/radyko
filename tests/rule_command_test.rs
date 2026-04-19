mod common;

#[cfg(test)]
mod rule_command_test {
    use std::io::Write;
    use std::path::PathBuf;

    use radyko::{app::config, cli::RuleArgs, commands::rule};
    use tempfile::NamedTempFile;

    #[tokio::test]
    #[ignore = "radiko apiに依存"]
    async fn run_test() -> anyhow::Result<()> {
        let mut config_tmp_file = NamedTempFile::new()?;
        writeln!(config_tmp_file, "{}", config::EXAMPLE_CONFIG)?;

        let args = RuleArgs {
            config: radyko::cli::ConfigArgs {
                config_path: PathBuf::from(config_tmp_file.path()),
            },
        };

        rule::run(args).await?;
        Ok(())
    }
}
