use thiserror::Error;

#[derive(Error, Debug)]
pub enum TreeBuilderError {
    #[error("Invalid leaf hash format: {0}")]
    InvalidLeafHash(String),

    #[error("Leaf not found in tree: {0}")]
    LeafNotFound(String),

    #[error("Empty leaf set provided")]
    EmptyLeaves,

    #[error("Invalid proof: {0}")]
    InvalidProof(String),

    #[error("Merkle tree not built")]
    TreeNotBuilt,

    #[error("Invalid tree depth: {0}")]
    InvalidDepth(usize),

    #[error("Poseidon hash error: {0}")]
    PoseidonError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Hex decode error: {0}")]
    HexError(#[from] hex::FromHexError),
}

pub type Result<T> = std::result::Result<T, TreeBuilderError>;
