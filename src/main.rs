use anyhow::Result;
use clap::Parser;
use codeskew::{Cli, Config, OutputGenerator};

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let cli = Cli::parse();

    // Create configuration from CLI arguments
    let config = Config::from_cli(&cli)?;

    // Validate the configuration
    config.validate()?;

    // Read the input file
    let code = std::fs::read_to_string(&config.input)?;

    // Create the pure WebGPU output generator
    let mut output_generator = OutputGenerator::new(config)
        .map_err(|e| anyhow::anyhow!("Failed to create WebGPU output generator: {}", e))?;

    // Generate gorgeous WebGPU output
    output_generator
        .generate(&code)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to generate WebGPU output: {}", e))?;

    Ok(())
}
