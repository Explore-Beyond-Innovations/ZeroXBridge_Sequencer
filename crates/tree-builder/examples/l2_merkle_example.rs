use num_bigint::BigUint;
use num_traits::Num;
use tree_builder::{MerkleTreeBuilder, Result, TreeBuilderError};

#[tokio::main]
pub async fn main() -> Result<()> {
    let leaf_one = felt252_to_hex(
        "3085182978037364507644541379307921604860861694664657935759708330416374536741",
    )?;
    let leaf_two = felt252_to_hex(
        "1515056012081702936544604035253985638654900467413915026150760243646139951112",
    )?;
    let leaf_three = felt252_to_hex(
        "2323060256672561756159719169078931556938075970039758487114302926228175567841",
    )?;
    let leaf_four = felt252_to_hex(
        "884555293850013781657518953358027212692898536740606299472615094634234324840",
    )?;

    let commitment_hashes = vec![leaf_one, leaf_two, leaf_three, leaf_four];
    let expected_root = felt252_to_hex(
        "423282815349921591262243120076891478879135827696329377607682678064132796520",
    )?;

    let mut leaves = Vec::new();
    for hash_str in &commitment_hashes {
        let bytes = hex::decode(hash_str)?;
        if bytes.len() != 32 {
            return Err(TreeBuilderError::InvalidLeafHash(format!(
                "Expected 32 bytes, got {}",
                bytes.len()
            )));
        }
        let mut array = [0u8; 32];
        array.copy_from_slice(&bytes);
        leaves.push(array);
    }

    let mut builder = MerkleTreeBuilder::new();
    builder.build_merkle(leaves).await?;

    let root = builder.get_root().await?;
    let root_hex = hex::encode(root);

    if expected_root != root_hex {
        println!(
            "Root mismatch! Expected: {}, Got: {}",
            expected_root, root_hex
        );
        std::process::exit(1);
    } else {
        println!("Root matches: {}", root_hex);
    }

    Ok(())
}

fn felt252_to_hex(felt: &str) -> Result<String> {
    let big_uint = BigUint::from_str_radix(felt, 10).map_err(|e| {
        TreeBuilderError::InvalidLeafHash(format!("Failed to parse felt252: {}", e))
    })?;

    let mut hex_str = format!("{:x}", big_uint);
    if hex_str.len() % 2 != 0 {
        hex_str = format!("0{}", hex_str);
    }
    while hex_str.len() < 64 {
        hex_str = format!("0{}", hex_str);
    }

    Ok(hex_str)
}
