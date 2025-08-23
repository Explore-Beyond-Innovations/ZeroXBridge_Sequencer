use serde::Serialize;
use std::fs::File;
use std::io::Write;
use std::path::Path;

#[derive(Serialize)]
struct Cairo1Input {
    data: Vec<Vec<u64>>,
}

pub fn generate_cairo1_inputs(
    commitment_hash: u64,
    proof_array: Vec<u64>,
    new_root: u64,
    output_dir: &str,
) -> Result<(), std::io::Error> {
    // Combine inputs into a single array
    let mut input_data = vec![commitment_hash];
    input_data.extend(proof_array);
    input_data.push(new_root);

    // Generate JSON file
    let json_data = Cairo1Input {
        data: vec![input_data.clone()],
    };
    let json_string = serde_json::to_string_pretty(&json_data)?;
    let json_path = Path::new(output_dir).join("input.cairo1.json");
    File::create(&json_path)?.write_all(json_string.as_bytes())?;

    // Generate TXT file
    let txt_string = input_data
        .iter()
        .map(|x| x.to_string())
        .collect::<Vec<String>>()
        .join(" ");
    let txt_content = format!("[{}]", txt_string);
    let txt_path = Path::new(output_dir).join("input.cairo1.txt");
    File::create(&txt_path)?.write_all(txt_content.as_bytes())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    #[test]
    fn test_generate_cairo1_inputs() {
        let commitment_hash = 12345;
        let proof_array = vec![67890, 111213];
        let new_root = 141516;
        let output_dir = "test_output";

        // Create temporary output directory
        fs::create_dir_all(output_dir).unwrap();

        // Generate files
        generate_cairo1_inputs(commitment_hash, proof_array.clone(), new_root, output_dir)
            .expect("Failed to generate files");

        // Verify JSON file
        let json_path = Path::new(output_dir).join("input.cairo1.json");
        let json_content = fs::read_to_string(&json_path).unwrap();
        let expected_json = r#"{
  "data": [
    [12345, 67890, 111213, 141516]
  ]
}"#;
        assert_eq!(json_content.trim(), expected_json.trim());

        // Verify TXT file
        let txt_path = Path::new(output_dir).join("input.cairo1.txt");
        let txt_content = fs::read_to_string(&txt_path).unwrap();
        let expected_txt = "[12345 67890 111213 141516]";
        assert_eq!(txt_content, expected_txt);

        // Clean up
        fs::remove_dir_all(output_dir).unwrap();
    }
}
