use thiserror::Error;

#[derive(Debug, Error)]
pub enum GeneratorError {
    #[error("Invalid password options: {0}")]
    InvalidOptions(String),

    #[error("Password length {0} is invalid (must be 5-128)")]
    InvalidLength(usize),

    #[error("Minimum character requirements ({0}) exceed password length ({1})")]
    RequirementsExceedLength(usize, usize),

    #[error("No character sets enabled")]
    NoCharacterSets,

    #[error("RNG failure: {0}")]
    RngError(String),

    #[error("Invalid passphrase options: {0}")]
    InvalidPassphraseOptions(String),

    #[error("Invalid passphrase word count {0} (must be 3-20)")]
    InvalidWordCount(usize),
}
