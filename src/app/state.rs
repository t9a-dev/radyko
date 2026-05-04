use std::{
    collections::HashSet,
    fs,
    io::Write,
    path::PathBuf,
    sync::{Arc, RwLock},
};

use secrecy::ExposeSecret;
use tempfile::TempDir;
use tracing::error;

use crate::{
    app::{
        config::{RadykoConfig, RecordingConfig},
        credential::RadikoCredential,
    },
    cli::{RecorderArgs, RuleArgs},
    model::{Program, program::ProgramId},
    radiko::RadikoClient,
};

#[derive(Debug)]
pub struct AppState {
    config: Arc<RwLock<RadykoConfig>>,
    pub radiko_client: RadikoClient,
}

impl AppState {
    pub async fn new(config: RadykoConfig, radiko_client: RadikoClient) -> anyhow::Result<Self> {
        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            radiko_client,
        })
    }

    pub async fn build_from_recorder_args(args: RecorderArgs) -> anyhow::Result<Self> {
        let radyko_config = RadykoConfig::parse_from_path(args.config.config_path)?;
        let radiko_credential = RadikoCredential::load_credential();
        let radiko_client = match radiko_credential {
            Some(c) => {
                RadikoClient::new_area_free(
                    c.email_address.expose_secret(),
                    c.password.expose_secret(),
                )
                .await?
            }
            None => RadikoClient::new().await?,
        };
        Self::new(radyko_config, radiko_client).await
    }

    pub async fn build_from_rule_args(args: RuleArgs) -> anyhow::Result<Self> {
        let radyko_config = RadykoConfig::parse_from_path(args.config.config_path)?;
        let radiko_credential = RadikoCredential::load_credential();
        let radiko_client = match radiko_credential {
            Some(c) => {
                RadikoClient::new_area_free(
                    c.email_address.expose_secret(),
                    c.password.expose_secret(),
                )
                .await?
            }
            None => RadikoClient::new().await?,
        };
        Self::new(radyko_config, radiko_client).await
    }

    pub fn config(&self) -> Arc<RwLock<RadykoConfig>> {
        Arc::clone(&self.config)
    }

    pub fn output_dir(&self) -> PathBuf {
        self.config.read().unwrap().recording.output_dir.clone()
    }

    pub fn schedule_update_interval_secs(&self) -> u64 {
        self.config
            .read()
            .unwrap()
            .recording
            .schedule_update_interval_secs
    }

    pub fn http_cache_dir() -> anyhow::Result<TempDir> {
        Ok(TempDir::new_in(".")?)
    }
}

#[derive(Debug)]
pub struct RecorderState {
    app_state: Arc<AppState>,
    inner: RecorderStateRef,
}

#[derive(Debug)]
struct RecorderStateRef {
    reserved_programs: Arc<RwLock<HashSet<ProgramId>>>,
    reserved_state_file_path: PathBuf,
}

impl RecorderState {
    pub fn new(app_state: Arc<AppState>, reserved_state_file_path: PathBuf) -> Self {
        let inner = RecorderStateRef {
            reserved_programs: Arc::new(RwLock::new(HashSet::new())),
            reserved_state_file_path,
        };
        Self { app_state, inner }
    }

    pub fn insert_reserved_program_id(&self, program: &Program) -> bool {
        let is_inserted = self
            .inner
            .reserved_programs
            .write()
            .unwrap()
            .insert(program.program_id());

        if !is_inserted && let Err(e) = self.add_reserved_program(program) {
            error!("add reserve program error: {:#?} ", e);
        }

        is_inserted
    }

    pub fn remove_reserved_program(&self, program_id: ProgramId) -> anyhow::Result<()> {
        self.inner
            .reserved_programs
            .write()
            .unwrap()
            .remove(&program_id);
        self.delete_reserved_program(program_id)
    }

    pub fn reload_config(&self, config_path: PathBuf) -> anyhow::Result<()> {
        let radyko_config = RadykoConfig::parse_from_path(config_path)?;
        let mut config_guard = self.app_state.config.write().unwrap();
        *config_guard = radyko_config;
        drop(config_guard);

        Ok(())
    }

    pub fn app_state(&self) -> Arc<AppState> {
        Arc::clone(&self.app_state)
    }

    pub fn config(&self) -> Arc<RwLock<RadykoConfig>> {
        Arc::clone(&self.app_state.config)
    }

    pub fn recording_config(&self) -> RecordingConfig {
        self.app_state.config.read().unwrap().recording.clone()
    }

    pub fn schedule_update_interval_secs(&self) -> u64 {
        self.app_state.schedule_update_interval_secs()
    }

    fn add_reserved_program(&self, program: &Program) -> anyhow::Result<()> {
        let mut file = std::fs::File::options()
            .create(true)
            .append(true)
            .open(self.inner.reserved_state_file_path.as_path())?;
        writeln!(file, "{} # {}", program.program_id(), program.get_info())?;
        Ok(())
    }

    fn delete_reserved_program(&self, program_id: ProgramId) -> anyhow::Result<()> {
        let reserved_programs = fs::read_to_string(self.inner.reserved_state_file_path.clone())?;
        let filtered = reserved_programs
            .lines()
            .filter(|line| !line.contains(&program_id.to_string()))
            .collect::<Vec<_>>()
            .join("\n");

        fs::write(self.inner.reserved_state_file_path.clone(), filtered)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{io::Read, ops::Not, sync::Arc};

    use chrono::{NaiveDateTime, TimeDelta, TimeZone};
    use chrono_tz::Asia::Tokyo;

    use crate::{
        app::state::{AppState, RecorderState},
        model::Program,
        test_helper::{load_example_config, radiko_client},
    };

    #[tokio::test]
    async fn add_reserve_program_test() -> anyhow::Result<()> {
        let app_state =
            Arc::new(AppState::new(load_example_config()?, radiko_client().await.clone()).await?);
        let mut reserved_programs_file = tempfile::NamedTempFile::new_in(".")?;
        let recorder_state =
            RecorderState::new(app_state, reserved_programs_file.path().to_path_buf());

        let on_air_duration = TimeDelta::hours(1);
        let start_at = Tokyo
            .from_local_datetime(
                &NaiveDateTime::parse_from_str("2000-01-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap(),
            )
            .unwrap();
        let end_at = start_at.checked_add_signed(on_air_duration).unwrap();
        let program = Program::new(start_at, end_at);

        recorder_state.add_reserved_program(&program)?;
        let mut content = String::new();
        reserved_programs_file.read_to_string(&mut content)?;
        assert!(content.is_empty().not());
        content.clear();

        recorder_state.remove_reserved_program(program.program_id())?;
        reserved_programs_file
            .reopen()?
            .read_to_string(&mut content)?;
        assert!(content.is_empty());

        Ok(())
    }
}
