use serde::{Deserialize, Serialize};

/// Database-specific proof structure for validating proof data before storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseProof {
    /// Index of the leaf in the Merkle tree
    pub leaf_index: usize,
    /// Array of sibling hashes for the proof path
    pub sibling_hashes: Vec<String>, // Hex strings for JSON serialization
    /// Peak bagging data (if applicable)
    #[serde(default)]
    pub peak_bagging: Vec<String>,
}

impl DatabaseProof {
    /// Validate that the proof has required fields and valid format
    pub fn validate(&self) -> Result<(), String> {
        // Validate sibling hashes are valid hex
        for (i, hash) in self.sibling_hashes.iter().enumerate() {
            if !hash.starts_with("0x") {
                return Err(format!("sibling_hashes[{}] must start with 0x", i));
            }
            if hash.len() != 66 { // 0x + 64 hex chars = 66 total
                return Err(format!("sibling_hashes[{}] must be exactly 66 characters (0x + 64 hex)", i));
            }
            if let Err(_) = hex::decode(&hash[2..]) {
                return Err(format!("sibling_hashes[{}] contains invalid hex characters", i));
            }
        }

        // Validate peak bagging if present
        for (i, hash) in self.peak_bagging.iter().enumerate() {
            if !hash.starts_with("0x") {
                return Err(format!("peak_bagging[{}] must start with 0x", i));
            }
            if hash.len() != 66 {
                return Err(format!("peak_bagging[{}] must be exactly 66 characters (0x + 64 hex)", i));
            }
            if let Err(_) = hex::decode(&hash[2..]) {
                return Err(format!("peak_bagging[{}] contains invalid hex characters", i));
            }
        }

        Ok(())
    }
}