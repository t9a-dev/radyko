use crate::domain::program::{Program, ProgramId};

pub trait ReservedProgramRepository: Sync + Send {
    fn reserved_program_ids(&self) -> anyhow::Result<Vec<ProgramId>>;

    fn save_reserved_programs(&self, programs: &[Program]) -> anyhow::Result<()>;

    fn delete_reserved_program(&self, program_id: ProgramId) -> anyhow::Result<()>;
}
