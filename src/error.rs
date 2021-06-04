#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error")]
    IoError(#[from] std::io::Error),

    #[error("Incomplete code, need {0} more bits")]
    Incomplete(usize),

    #[error("Bad Code received: {0}")]
    BadCode(u16),
}
