use anyhow::Result;
use reqwest::multipart::{Form, Part};
use reqwest::Client;
use std::env;
use std::fs;

pub async fn submit_sharp_proof_job(
    api_key: String,
    result: String,
    program_path: String,
    input_path: String,
) -> Result<()> {
    let program_bytes = fs::read(program_path)?;
    let input_bytes = fs::read(input_path)?;

    let form = Form::new()
        .text("layout", "auto")
        .text("cairoVm", "rust")
        .text("cairoVersion", "cairo1")
        .text("mockFactHash", "false")
        .text("declaredJobSize", "S")
        .text("direction", result)
        .part(
            "program",
            Part::bytes(program_bytes).file_name("cairo1.sierra.json"),
        )
        .part(
            "input",
            Part::bytes(input_bytes).file_name("input.cairo1.txt"),
        );

    let client = Client::new();

    let url = if let Ok(endpoint) = env::var("ATLANTIC_API_ENDPOINT") {
        format!("{}?apiKey={}", endpoint, api_key)
    } else {
        format!(
            "https://staging.atlantic.api.herodotus.cloud/atlantic-query?apiKey={}",
            api_key
        )
    };

    let response = client.post(&url).multipart(form).send().await?;

    if response.status().is_success() {
        let resp_text = response.text().await?;
        println!("Success: {}", resp_text);
    } else {
        let status = response.status();
        let resp_text = response.text().await?;
        eprintln!("Error ({}): {}", status, resp_text);
    }
    Ok(())
}
