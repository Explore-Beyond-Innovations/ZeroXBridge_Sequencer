use anyhow::Result;
use mockito::{mock, Matcher};
use std::env;
use std::fs;
use tokio;
use zeroxbridge_sequencer::http::client::submit_sharp_proof_job;

fn setup_dummy_files() -> Result<()> {
    fs::create_dir_all("tmp/target/dev")?;
    fs::create_dir_all("tmp")?;
    fs::write("tmp/target/dev/cairo1.sierra.json", r#"{"dummy":"data"}"#)?;
    fs::write("tmp/input.cairo1.txt", "dummy input")?;
    Ok(())
}

#[tokio::test]
async fn test_submit_sharp_proof_job_positive_l1() -> Result<()> {
    setup_dummy_files()?;
    env::set_var(
        "ATLANTIC_API_ENDPOINT",
        format!("{}/atlantic-query", mockito::server_url()),
    );
    let m = mock("POST", Matcher::Any)
        .match_query(Matcher::UrlEncoded("apiKey".into(), "test_api".into()))
        .with_status(200)
        .with_body("Job submitted successfully")
        .create();

    let res = submit_sharp_proof_job(
        "test_api".into(),
        "PROOF_VERIFICATION_ON_L1".into(),
        "tmp/target/dev/cairo1.sierra.json".into(),
        "tmp/input.cairo1.txt".into(),
    )
    .await;
    assert!(res.is_ok());
    m.assert();
    Ok(())
}

#[tokio::test]
async fn test_submit_sharp_proof_job_positive_l2() -> Result<()> {
    setup_dummy_files()?;
    env::set_var(
        "ATLANTIC_API_ENDPOINT",
        format!("{}/atlantic-query", mockito::server_url()),
    );
    let m = mock("POST", Matcher::Any)
        .match_query(Matcher::UrlEncoded("apiKey".into(), "test_api".into()))
        .with_status(200)
        .with_body("Job submitted successfully")
        .create();

    let res = submit_sharp_proof_job(
        "test_api".into(),
        "PROOF_VERIFICATION_ON_L2".into(),
        "tmp/target/dev/cairo1.sierra.json".into(),
        "tmp/input.cairo1.txt".into(),
    )
    .await;
    assert!(res.is_ok());
    m.assert();
    Ok(())
}

#[tokio::test]
async fn test_submit_sharp_proof_job_negative_l1() -> Result<()> {
    setup_dummy_files()?;
    env::set_var(
        "ATLANTIC_API_ENDPOINT",
        format!("{}/atlantic-query", mockito::server_url()),
    );
    let m = mock("POST", Matcher::Any)
        .match_query(Matcher::UrlEncoded("apiKey".into(), "bad_api".into()))
        .with_status(400)
        .with_body("Invalid API key")
        .create();

    let res = submit_sharp_proof_job(
        "bad_api".into(),
        "PROOF_VERIFICATION_ON_L1".into(),
        "tmp/target/dev/cairo1.sierra.json".into(),
        "tmp/input.cairo1.txt".into(),
    )
    .await;
    assert!(res.is_ok());
    m.assert();
    Ok(())
}

#[tokio::test]
async fn test_submit_sharp_proof_job_negative_l2() -> Result<()> {
    setup_dummy_files()?;
    env::set_var(
        "ATLANTIC_API_ENDPOINT",
        format!("{}/atlantic-query", mockito::server_url()),
    );
    let m = mock("POST", Matcher::Any)
        .match_query(Matcher::UrlEncoded("apiKey".into(), "bad_api".into()))
        .with_status(400)
        .with_body("Invalid API key")
        .create();

    let res = submit_sharp_proof_job(
        "bad_api".into(),
        "PROOF_VERIFICATION_ON_L2".into(),
        "tmp/target/dev/cairo1.sierra.json".into(),
        "tmp/input.cairo1.txt".into(),
    )
    .await;
    assert!(res.is_ok());
    m.assert();
    Ok(())
}
