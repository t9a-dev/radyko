use std::{
    collections::HashSet,
    fs,
    io::{BufWriter, Write},
    path::PathBuf,
    sync::{Arc, RwLock},
};

use chrono::{DateTime, Utc};
use chrono_tz::{Asia::Tokyo, Tz};
use secrecy::ExposeSecret;
use tempfile::TempDir;
use tracing::error;

use crate::{
    app::{
        config::{RadykoConfig, RecordingConfig},
        credential::RadikoCredential,
    },
    cli::{RecorderArgs, RuleArgs},
    model::{
        Program,
        program::{EndAt, ProgramId},
    },
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
        self.config
            .read()
            .expect("config RwLock poisoned")
            .recording
            .output_dir
            .clone()
    }

    pub fn schedule_update_interval_secs(&self) -> u64 {
        self.config
            .read()
            .expect("config RwLock poisoned")
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
        let _ = fs::File::create_new(reserved_state_file_path.as_path());
        let inner = RecorderStateRef {
            reserved_programs: Arc::new(RwLock::new(HashSet::new())),
            reserved_state_file_path,
        };
        Self { app_state, inner }
    }

    pub fn collect_aired_program_ids(
        &self,
        now: Option<DateTime<Tz>>,
    ) -> anyhow::Result<Vec<ProgramId>> {
        let now = now.unwrap_or(Utc::now().with_timezone(&Tokyo));

        Ok(self
            .get_reserved_program_ids()?
            .into_iter()
            .filter(|p| {
                let EndAt(end_at) = p.2;
                end_at < now
            })
            .collect())
    }

    pub fn add_reserve_programs(&self, programs: Vec<Program>) -> Vec<Program> {
        let mut reserved_programs_guard = self
            .inner
            .reserved_programs
            .write()
            .expect("reserved_programs RwLock poisoned");
        let reserved_programs = programs
            .into_iter()
            .filter(|program| reserved_programs_guard.insert(program.program_id()))
            .collect::<Vec<_>>();

        if let Err(e) = self.append_reserved_program(&reserved_programs) {
            error!("add reserve program error: {:#?} ", e);
        }

        reserved_programs
    }

    pub fn remove_reserved_program(&self, program_id: ProgramId) -> anyhow::Result<()> {
        self.inner
            .reserved_programs
            .write()
            .expect("reserved_programs RwLock poisoned")
            .remove(&program_id);
        self.delete_reserved_program(program_id)
    }

    pub fn reload_config(&self, config_path: PathBuf) -> anyhow::Result<()> {
        let radyko_config = RadykoConfig::parse_from_path(config_path)?;
        let mut config_guard = self
            .app_state
            .config
            .write()
            .expect("app_state config RwLock poisoned");
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
        self.app_state
            .config
            .read()
            .expect("app_state.config RwLock poisoned")
            .recording
            .clone()
    }

    pub fn schedule_update_interval_secs(&self) -> u64 {
        self.app_state.schedule_update_interval_secs()
    }

    fn get_reserved_program_ids(&self) -> anyhow::Result<Vec<ProgramId>> {
        ProgramId::parse_from_string(fs::read_to_string(
            self.inner.reserved_state_file_path.clone(),
        )?)
    }

    fn append_reserved_program(&self, programs: &[Program]) -> anyhow::Result<()> {
        let reserved_program_ids = ProgramId::parse_from_string(fs::read_to_string(
            self.inner.reserved_state_file_path.as_path(),
        )?)?;
        let reserve_programs = programs
            .iter()
            .filter(|program| !reserved_program_ids.contains(&program.program_id()))
            .collect::<Vec<_>>();

        let mut file = BufWriter::new(
            std::fs::File::options()
                .create(true)
                .append(true)
                .open(self.inner.reserved_state_file_path.as_path())?,
        );
        for program in reserve_programs {
            writeln!(file, "{} # {}", program.program_id(), program.get_info())?;
        }
        file.flush()?;

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
    use std::{io::Read, path::PathBuf, sync::Arc};

    use chrono::{NaiveDateTime, TimeDelta, TimeZone};
    use chrono_tz::Asia::Tokyo;

    use crate::{
        app::state::{AppState, RecorderState},
        model::{
            Program,
            program::{ProgramId, StationId},
        },
        test_helper::{load_example_config, radiko_client},
    };

    async fn setup_recorder_state(
        reserved_programs_file_path: PathBuf,
    ) -> anyhow::Result<RecorderState> {
        let app_state =
            Arc::new(AppState::new(load_example_config()?, radiko_client().await.clone()).await?);
        Ok(RecorderState::new(app_state, reserved_programs_file_path))
    }

    #[tokio::test]
    async fn get_reserved_program_test() -> anyhow::Result<()> {
        let reserved_programs_file = tempfile::NamedTempFile::new_in(".")?;
        let recorder_state =
            setup_recorder_state(reserved_programs_file.path().to_path_buf()).await?;

        let on_air_duration = TimeDelta::hours(1);
        let start_at = Tokyo
            .from_local_datetime(
                &NaiveDateTime::parse_from_str("2000-01-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap(),
            )
            .unwrap();
        let end_at = start_at.checked_add_signed(on_air_duration).unwrap();
        let mut program = Program::new(start_at, end_at);
        program.station_id = "LFR".to_string();

        // 録音予約を永続化(LFR)
        recorder_state.append_reserved_program(&vec![program])?;

        // 録音予約を全て取得
        let all_reserved_program_ids = recorder_state.get_reserved_program_ids()?;
        assert_eq!(all_reserved_program_ids.iter().count(), 1);
        assert_eq!(
            all_reserved_program_ids.first().unwrap().0,
            StationId("LFR".to_string())
        );

        // 放送が終了していない番組情報は取得できない
        let now = Tokyo
            .from_local_datetime(
                &NaiveDateTime::parse_from_str("1999-04-02 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap(),
            )
            .unwrap();
        let program_ids = recorder_state.collect_aired_program_ids(Some(now))?;
        assert!(program_ids.is_empty());

        // 放送が終了している番組情報が取得できる
        let now = Tokyo
            .from_local_datetime(
                &NaiveDateTime::parse_from_str("2000-04-02 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap(),
            )
            .unwrap();
        let program_ids = recorder_state.collect_aired_program_ids(Some(now))?;
        assert_eq!(program_ids.iter().count(), 1);
        assert_eq!(program_ids.first().unwrap().0, StationId("LFR".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn add_reserve_program_test() -> anyhow::Result<()> {
        let mut reserved_programs_file = tempfile::NamedTempFile::new_in(".")?;
        let recorder_state =
            setup_recorder_state(reserved_programs_file.path().to_path_buf()).await?;

        let on_air_duration = TimeDelta::hours(1);
        let start_at = Tokyo
            .from_local_datetime(
                &NaiveDateTime::parse_from_str("2000-01-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap(),
            )
            .unwrap();
        let end_at = start_at.checked_add_signed(on_air_duration).unwrap();
        let mut program = Program::new(start_at, end_at);
        program.station_id = "LFR".to_string();

        // 録音予約を永続化(LFR)
        recorder_state.append_reserved_program(&vec![program.clone()])?;
        let mut content = String::new();
        reserved_programs_file.read_to_string(&mut content)?;
        assert_eq!(ProgramId::parse_from_string(content)?.len(), 1);

        // 重複した予約情報は登録されない(LFR)
        recorder_state.append_reserved_program(&vec![program.clone()])?;
        let mut content = String::new();
        reserved_programs_file
            .reopen()?
            .read_to_string(&mut content)?;
        assert_eq!(ProgramId::parse_from_string(content)?.len(), 1);

        // 別の放送局(TBS)情報を指定して予約情報を削除
        // 録音予約(LFR)が残っている
        program.station_id = "TBS".to_string();
        recorder_state.remove_reserved_program(program.program_id())?;
        let mut content = String::new();
        reserved_programs_file
            .reopen()?
            .read_to_string(&mut content)?;
        assert_eq!(
            ProgramId::parse_from_string(content)?.first().unwrap().0,
            StationId("LFR".to_string())
        );

        // 録音が完了したので予約情報を削除
        program.station_id = "LFR".to_string();
        recorder_state.remove_reserved_program(program.program_id())?;
        let mut content = String::new();
        reserved_programs_file
            .reopen()?
            .read_to_string(&mut content)?;
        assert!(content.is_empty());
        content.clear();

        Ok(())
    }
}
