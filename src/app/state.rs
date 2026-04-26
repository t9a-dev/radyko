use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{Arc, RwLock},
};

use secrecy::ExposeSecret;
use tempfile::TempDir;

use crate::{
    app::{
        config::{RadykoConfig, RecordingConfig},
        credential::RadikoCredential,
    },
    cli::{RecorderArgs, RuleArgs},
    model::program::ProgramId,
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
    inner: Arc<RwLock<RecorderStateRef>>,
}

#[derive(Debug)]
struct RecorderStateRef {
    reserved_programs: HashSet<ProgramId>,
}

impl RecorderState {
    pub fn new(app_state: Arc<AppState>) -> Self {
        let inner = RecorderStateRef {
            reserved_programs: HashSet::new(),
        };
        Self {
            app_state,
            inner: Arc::new(RwLock::new(inner)),
        }
    }

    pub fn insert_reserved_program_id(&self, program_id: ProgramId) -> bool {
        self.inner
            .write()
            .unwrap()
            .reserved_programs
            .insert(program_id)
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
}
