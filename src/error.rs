#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("beamterm: {0}")]
    Beamterm(#[from] beamterm_core::Error),
    #[error("{0}")]
    Other(String),
}
