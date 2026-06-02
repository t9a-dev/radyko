use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{Arc, RwLock},
};

use jiff::Zoned;
use tracing::error;

use crate::{application::credential::RadikoCredential, domain::program::RecordingDurationBuffers};
use crate::{
    application::{
        config::RadykoConfig,
        port::{RadikoClient, ReservedProgramRepository},
        utils::Utils,
    },
    cli::{RecorderArgs, RuleArgs},
    domain::program::{Program, ProgramId, RadykoDateTime},
    radiko::new_radiko_client,
};

pub struct AppState {
    config: Arc<RwLock<RadykoConfig>>,
    radiko_client: Arc<dyn RadikoClient>,
}

impl AppState {
    pub async fn new(
        config: RadykoConfig,
        radiko_client: Arc<dyn RadikoClient>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            radiko_client,
        })
    }

    pub async fn build_from_recorder_args(args: RecorderArgs) -> anyhow::Result<Self> {
        let radyko_config = RadykoConfig::parse_from_path(args.config.config_path)?;
        let radiko_credential = RadikoCredential::load_from_env_file();
        let radiko_client = new_radiko_client(radiko_credential).await?;
        Self::new(radyko_config, radiko_client).await
    }

    pub async fn build_from_rule_args(args: RuleArgs) -> anyhow::Result<Self> {
        let radyko_config = RadykoConfig::parse_from_path(args.config.config_path)?;
        let radiko_credential = RadikoCredential::load_from_env_file();
        let radiko_client = new_radiko_client(radiko_credential).await?;
        Self::new(radyko_config, radiko_client).await
    }

    pub fn reload_config(&self, config_path: PathBuf) -> anyhow::Result<()> {
        let radyko_config = RadykoConfig::parse_from_path(config_path)?;

        // writer guardの生存期間を短くしたいのでスコープを明示的に切る
        {
            let mut config_guard = self
                .config
                .write()
                .expect("reserved_programs RwLock poisoned");
            *config_guard = radyko_config;
        }

        Ok(())
    }

    pub fn radiko_client(&self) -> Arc<dyn RadikoClient> {
        Arc::clone(&self.radiko_client)
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

    pub fn recording_duration_buffers(&self) -> RecordingDurationBuffers {
        RecordingDurationBuffers::from_config(
            self.config
                .read()
                .expect("config RwLock poisoned")
                .recording
                .duration_buffer_secs
                .clone(),
        )
    }
}

pub struct RecorderState {
    inner: RecorderStateRef,
}

struct RecorderStateRef {
    reserved_programs: Arc<RwLock<HashSet<ProgramId>>>,
    reserved_program_repository: Arc<dyn ReservedProgramRepository>,
}

impl RecorderState {
    pub fn new(reserved_program_repository: Arc<dyn ReservedProgramRepository>) -> Self {
        let inner = RecorderStateRef {
            reserved_programs: Arc::new(RwLock::new(HashSet::new())),
            reserved_program_repository,
        };
        Self { inner }
    }

    pub fn collect_aired_program_ids(&self, now: Option<Zoned>) -> anyhow::Result<Vec<ProgramId>> {
        let now = now.unwrap_or(Utils::now_in_tz_tokyo());

        Ok(self
            .inner
            .reserved_program_repository
            .reserved_program_ids()?
            .into_iter()
            .filter(|p| p.end_at().date() < now)
            .collect())
    }

    pub fn add_reserve_programs(&self, programs: Vec<Program>) -> Vec<Program> {
        // writer guardの生存期間を短くしたいのでスコープを明示的に切る
        let reserved_programs = {
            let mut reserved_programs_guard = self
                .inner
                .reserved_programs
                .write()
                .expect("reserved_programs RwLock poisoned");
            programs
                .into_iter()
                .filter(|program| reserved_programs_guard.insert(program.program_id()))
                .collect::<Vec<_>>()
        };

        if let Err(e) = self
            .inner
            .reserved_program_repository
            .save_reserved_programs(&reserved_programs)
        {
            error!("add reserve program error: {:#?} ", e);
        }

        reserved_programs
    }

    pub fn remove_reserved_program(&self, program_id: ProgramId) -> anyhow::Result<()> {
        {
            self.inner
                .reserved_programs
                .write()
                .expect("reserved_programs RwLock poisoned")
                .remove(&program_id);
        }
        self.inner
            .reserved_program_repository
            .delete_reserved_program(program_id)
    }
}

#[cfg(test)]
mod tests {
    use std::{io::Read, path::PathBuf};

    use jiff::{ToSpan, civil::DateTime};

    use crate::{
        RADYKO_TZ_NAME,
        application::state::RecorderState,
        domain::program::{EndAt, Program, ProgramId, RadykoDateTime, StartAt, StationId},
        infrastructure::new_file_reserved_repository,
    };

    const DATETIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

    async fn setup_recorder_state(
        reserved_programs_file_path: PathBuf,
    ) -> anyhow::Result<RecorderState> {
        Ok(RecorderState::new(new_file_reserved_repository(
            reserved_programs_file_path,
        )))
    }

    #[tokio::test]
    #[ignore = "radiko apiに依存"]
    async fn get_reserved_program_test() -> anyhow::Result<()> {
        let reserved_programs_file = tempfile::NamedTempFile::new_in(".")?;
        let recorder_state =
            setup_recorder_state(reserved_programs_file.path().to_path_buf()).await?;

        let on_air_duration = 1.hours();
        let dummy_start_at =
            DateTime::strptime(DATETIME_FORMAT, "2000-01-01 00:00:00")?.in_tz(RADYKO_TZ_NAME)?;
        let dummy_end_at = dummy_start_at.checked_add(on_air_duration).unwrap();
        let dummy_title = "オールナイトニッポン".to_string();
        let dummy_performer = "フワちゃん".to_string();
        let program = Program::new(
            ProgramId::new(
                StationId::new("LFR".to_string()),
                StartAt::new(dummy_start_at),
                EndAt::new(dummy_end_at),
            ),
            dummy_title,
            dummy_performer,
        );

        // 録音予約を永続化(LFR)
        recorder_state.add_reserve_programs(vec![program]);

        // 過去（放送済み）の録音予約を全て取得
        let aired_reserve_program_ids = recorder_state.collect_aired_program_ids(Some(
            DateTime::strptime(DATETIME_FORMAT, "2100-01-01 00:00:00")?.in_tz(RADYKO_TZ_NAME)?,
        ))?;
        assert_eq!(aired_reserve_program_ids.len(), 1);
        assert_eq!(
            *aired_reserve_program_ids.first().unwrap().station_id(),
            StationId::new("LFR".to_string())
        );

        // 放送が終了していない番組情報は取得できない
        let now =
            DateTime::strptime(DATETIME_FORMAT, "1999-04-02 00:00:00")?.in_tz(RADYKO_TZ_NAME)?;
        let program_ids = recorder_state.collect_aired_program_ids(Some(now))?;
        assert!(program_ids.is_empty());

        // 放送が終了している番組情報が取得できる
        let now =
            DateTime::strptime(DATETIME_FORMAT, "2000-04-02 00:00:00")?.in_tz(RADYKO_TZ_NAME)?;
        let program_ids = recorder_state.collect_aired_program_ids(Some(now))?;
        assert_eq!(program_ids.len(), 1);
        assert_eq!(
            *program_ids.first().unwrap().station_id(),
            StationId::new("LFR".to_string())
        );

        Ok(())
    }

    #[tokio::test]
    #[ignore = "radiko apiに依存"]
    async fn add_reserve_program_test() -> anyhow::Result<()> {
        let mut reserved_programs_file = tempfile::NamedTempFile::new_in(".")?;
        let recorder_state =
            setup_recorder_state(reserved_programs_file.path().to_path_buf()).await?;

        let on_air_duration = 1.hours();
        let start_at =
            DateTime::strptime(DATETIME_FORMAT, "2000-01-01 00:00:00")?.in_tz(RADYKO_TZ_NAME)?;
        let end_at = start_at.checked_add(on_air_duration).unwrap();
        let dummy_title = "オールナイトニッポン".to_string();
        let dummy_performer = "フワちゃん".to_string();
        let program = Program::new(
            ProgramId::new(
                StationId::new("LFR".to_string()),
                StartAt::new(start_at.clone()),
                EndAt::new(end_at.clone()),
            ),
            dummy_title.clone(),
            dummy_performer.clone(),
        );

        // 録音予約を永続化(LFR)
        recorder_state.add_reserve_programs(vec![program.clone()]);
        let mut content = String::new();
        reserved_programs_file.read_to_string(&mut content)?;
        assert_eq!(ProgramId::parse_from_string(content)?.len(), 1);

        // 重複した予約情報は登録されない(LFR)
        recorder_state.add_reserve_programs(vec![program.clone()]);
        let mut content = String::new();
        reserved_programs_file
            .reopen()?
            .read_to_string(&mut content)?;
        assert_eq!(ProgramId::parse_from_string(content)?.len(), 1);

        // 別の放送局(TBS)情報を指定して予約情報を削除
        // 録音予約(LFR)が残っている
        let program = Program::new(
            ProgramId::new(
                StationId::new("TBS".to_string()),
                StartAt::new(start_at.clone()),
                EndAt::new(end_at.clone()),
            ),
            dummy_title.clone(),
            dummy_performer.clone(),
        );
        recorder_state.remove_reserved_program(program.program_id())?;
        let mut content = String::new();
        reserved_programs_file
            .reopen()?
            .read_to_string(&mut content)?;
        assert_eq!(
            *ProgramId::parse_from_string(content)?
                .first()
                .unwrap()
                .station_id(),
            StationId::new("LFR".to_string())
        );

        // 録音が完了したので"LFR"予約情報を削除
        let program = Program::new(
            ProgramId::new(
                StationId::new("LFR".to_string()),
                StartAt::new(start_at),
                EndAt::new(end_at),
            ),
            dummy_title,
            dummy_performer,
        );
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
