use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn run_scarb_build(project_path: &str) -> Result<PathBuf, String> {
    let target_dir = Path::new(project_path);

    if !target_dir.exists() {
        return Err(format!("No Scarb project in this directory: {}", project_path));
    }

    println!("Building Scarb project at {:?}", target_dir);

    let status = Command::new("scarb")
        .arg("build")
        .current_dir(&target_dir)
        .status();

    match status {
        Ok(status) if status.success() => {
            // we need to check the .toml file of the project
            // so we can get the package name and compute its file/out folder
            let scarb_toml_path = target_dir.join("Scarb.toml");
            let toml_str = fs::read_to_string(&scarb_toml_path)
                .map_err(|e| format!("Failed to read Scarb.toml: {}", e))?;
            let parsed: toml::Value = toml_str
                .parse()
                .map_err(|e| format!("Failed to parse Scarb.toml: {}", e))?;

            let package_name = parsed
                .get("package")
                .and_then(|pkg| pkg.get("name"))
                .and_then(|name| name.as_str())
                .ok_or("Could not find package.name in Scarb.toml")?;

            let output_file = target_dir
                .join("target/dev")
                .join(format!("{}.sierra.json", package_name));

            if output_file.exists() {
                println!("Output file found: {:?}", output_file);
                Ok(output_file)
            } else {
                Err(format!("❌ Output file not found: {:?}", output_file))
            }
        }
        Ok(status) => Err(format!("❌ Build failed. Exit code: {:?}", status.code())),
        Err(err) => Err(format!("❌ Failed to execute Scarb: {}", err)),
    }
}
