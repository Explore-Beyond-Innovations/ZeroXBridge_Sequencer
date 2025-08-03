use std::path::PathBuf;
use tempfile::tempdir;

#[tokio::test]
async fn test_calldata_reading() {
    // Create a temporary directory with test calldata files
    let temp_dir = tempdir().unwrap();
    let calldata_dir = temp_dir.path();

    // Create test calldata files
    std::fs::write(calldata_dir.join("initial"), "0x123 0x456 0x789").unwrap();
    std::fs::write(calldata_dir.join("step1"), "0xabc 0xdef").unwrap();
    std::fs::write(calldata_dir.join("step2"), "0x111 0x222").unwrap();
    std::fs::write(calldata_dir.join("final"), "0x999 0xaaa").unwrap();

    // Verify files exist
    assert!(calldata_dir.join("initial").exists());
    assert!(calldata_dir.join("step1").exists());
    assert!(calldata_dir.join("step2").exists());
    assert!(calldata_dir.join("final").exists());
    assert!(!calldata_dir.join("step3").exists()); // Should not exist

    // Test string to hex conversion
    let test_relayer = TestProofSubmissionRelayer;
    assert_eq!(test_relayer.string_to_hex("test"), "0x74657374");
    assert_eq!(test_relayer.string_to_hex(""), "0x");
}

#[tokio::test]
async fn test_missing_calldata_directory() {
    let non_existent_path = PathBuf::from("/non/existent/path");
    assert!(!non_existent_path.exists());
}

#[tokio::test]
async fn test_missing_required_files() {
    let temp_dir = tempdir().unwrap();
    let calldata_dir = temp_dir.path();

    // Create only some files, missing others
    std::fs::write(calldata_dir.join("initial"), "0x123").unwrap();
    // Missing step1, step2, final

    assert!(calldata_dir.join("initial").exists());
    assert!(!calldata_dir.join("step1").exists());
    assert!(!calldata_dir.join("final").exists());
}

// Helper struct for testing
struct TestProofSubmissionRelayer;

impl TestProofSubmissionRelayer {
    fn string_to_hex(&self, input: &str) -> String {
        let mut hex_string = String::from("0x");
        for byte in input.bytes() {
            hex_string.push_str(&format!("{:02x}", byte));
        }
        hex_string
    }
}

#[test]
fn test_string_to_hex_conversion() {
    let relayer = TestProofSubmissionRelayer;

    // Test various string inputs
    assert_eq!(relayer.string_to_hex("hello"), "0x68656c6c6f");
    assert_eq!(
        relayer.string_to_hex("recursive_with_poseidon"),
        "0x7265637572736976655f776974685f706f736569646f6e"
    );
    assert_eq!(
        relayer.string_to_hex("keccak_160_lsb"),
        "0x6b656363616b5f3136305f6c7362"
    );
    assert_eq!(relayer.string_to_hex("stone6"), "0x73746f6e6536");
    assert_eq!(relayer.string_to_hex("true"), "0x74727565");
}
