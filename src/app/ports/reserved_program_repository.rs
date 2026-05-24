use crate::radiko::model::program::{Program, ProgramId};

pub trait ReservedProgramRepository: Sync + Send {
    fn get_reserved_program_ids(&self) -> anyhow::Result<Vec<ProgramId>>;

    fn append_reserved_program(&self, programs: &[Program]) -> anyhow::Result<()>;

    fn delete_reserved_program(&self, program_id: ProgramId) -> anyhow::Result<()>;
}
