use std::{
    fs,
    io::{BufWriter, Write},
    path::PathBuf,
    sync::Arc,
};

use crate::{
    app::ports,
    domain::program::{Program, ProgramId},
};

pub fn new_file_reserved_repository(
    reserved_state_file_path: PathBuf,
) -> Arc<dyn ports::ReservedProgramRepository> {
    let _ = fs::File::create_new(reserved_state_file_path.as_path());
    Arc::new(FileReservedProgramRepository {
        reserved_state_file_path,
    })
}

struct FileReservedProgramRepository {
    reserved_state_file_path: PathBuf,
}

impl ports::ReservedProgramRepository for FileReservedProgramRepository {
    fn reserved_program_ids(&self) -> anyhow::Result<Vec<ProgramId>> {
        ProgramId::parse_from_string(fs::read_to_string(self.reserved_state_file_path.clone())?)
    }

    fn save_reserved_programs(&self, programs: &[Program]) -> anyhow::Result<()> {
        let reserved_program_ids = ProgramId::parse_from_string(fs::read_to_string(
            self.reserved_state_file_path.as_path(),
        )?)?;
        let reserve_programs = programs
            .iter()
            .filter(|program| !reserved_program_ids.contains(&program.program_id()))
            .collect::<Vec<_>>();

        let mut file = BufWriter::new(
            std::fs::File::options()
                .create(true)
                .append(true)
                .open(self.reserved_state_file_path.as_path())?,
        );
        for program in reserve_programs {
            writeln!(file, "{} # {}", program.program_id(), program.info())?;
        }
        file.flush()?;

        Ok(())
    }

    fn delete_reserved_program(&self, program_id: ProgramId) -> anyhow::Result<()> {
        let reserved_programs = fs::read_to_string(self.reserved_state_file_path.clone())?;
        let filtered = reserved_programs
            .lines()
            .filter(|line| !line.contains(&program_id.to_string()))
            .collect::<Vec<_>>()
            .join("\n");

        fs::write(self.reserved_state_file_path.clone(), filtered)?;
        Ok(())
    }
}
