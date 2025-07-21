use accumulators::{
    mmr::MMRError,
    store::{InStoreTableError, StoreError},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TreeBuilderError {
    #[error(transparent)]
    MMRError(#[from] MMRError),
    #[error(transparent)]
    StoreError(#[from] StoreError),
    #[error(transparent)]
    TableError(#[from] InStoreTableError),
    #[error("Failed to decode hex: {0}")]
    HexError(String),
    #[error("Failed to convert to array: {0}")]
    ConversionError(String),
    #[error("Invalid leaf hash: {0}")]
    InvalidLeafHash(String),
    #[error(transparent)]
    FromHexError(#[from] hex::FromHexError),
}
