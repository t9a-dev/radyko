use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProgramParseError {
    #[error("{}",.0)]
    Invalid(String),
}
