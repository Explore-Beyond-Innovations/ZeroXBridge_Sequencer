use thiserror::Error;

#[derive(Debug, Error)]
pub enum TreeBuilderError {
    #[error("Failed to decode hex: {0}")]
    HexError(String),
    #[error("Failed to convert to array: {0}")]
    ConversionError(String),
    #[error("Invalid leaf hash: {0}")]
    InvalidLeafHash(String),
    #[error("Tree computation error: {0}")]
    TreeError(String),
    #[error("Invalid proof: {0}")]
    ProofError(String),
    #[error(transparent)]
    FromHexError(#[from] hex::FromHexError),
}
