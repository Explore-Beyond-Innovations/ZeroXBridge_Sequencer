#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use tempfile::tempdir;
    use zeroxbridge_sequencer::proof_client::proof_generator::run_scarb_build;

    #[test]
    fn test_run_scarb_build_pass() {
        let tmp_dir = tempdir().expect("Failed to create temporary directory");
        let project_path = tmp_dir
            .path()
            .to_str()
            .expect("Failed to convert path to string");

        // we'll create a temp directory that'll eventually be deleted when this test
        // goes out of scope so git won't track it.
        let _ = fs::create_dir_all(Path::new(project_path).join("src"));
        fs::write(
            Path::new(project_path).join("Scarb.toml"),
            r#"
[package]
name = "test_project"
version = "0.1.0"

[dependencies]
"#,
        )
        .unwrap();
        fs::write(
            Path::new(project_path).join("src/lib.cairo"),
            r#"
fn main() {
  println!("Hello people of Cairo!!")
}
"#,
        )
        .unwrap();

        let result = run_scarb_build(project_path);
        assert!(
            result.is_ok(),
            "Scarb build should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_run_scarb_build_fail() {
        let result = run_scarb_build("non_existent_path");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_lowercase()
                .contains("no scarb project"),
            "Expected missing project error"
        );
    }
}
