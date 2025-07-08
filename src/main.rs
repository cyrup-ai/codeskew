use anyhow::Result;
use clap::Parser;
use codeskew::{Cli, Config, OutputGenerator};
use env_logger::{Builder, Target};
use log::{debug, info, warn};
use std::io::Write;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize env_logger with custom formatting for pipeline debugging
    Builder::from_default_env()
        .target(Target::Stderr)
        .format(|buf, record| {
            let thread_id = std::thread::current().id();
            let module_path = record.module_path().unwrap_or("unknown");
            writeln!(
                buf,
                "[{} {:5} {:?} {}:{}] {}",
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                record.level(),
                thread_id,
                module_path,
                record.line().unwrap_or(0),
                record.args()
            )
        })
        .init();

    info!("🚀 CodeSkew pipeline starting - comprehensive DEBUG logging enabled");
    debug!("📋 Pipeline Stage 1: CLI argument parsing");

    // Parse command line arguments
    let cli = Cli::parse();
    debug!("✅ CLI arguments parsed successfully: input={:?}, output={:?}",
           cli.input.display(),
           cli.output.display());

    debug!("📋 Pipeline Stage 2: Configuration creation and validation");

    // Create configuration from CLI arguments
    let config = Config::from_cli(&cli)?;
    debug!("✅ Configuration created: width={}x{}, format={:?}, theme={:?}",
           config.width, config.height, config.format, config.theme);

    // Validate the configuration
    config.validate()?;
    debug!("✅ Configuration validation passed");

    debug!("📋 Pipeline Stage 3: Input file reading");

    // Read the input file
    let code = std::fs::read_to_string(&config.input)?;
    debug!("✅ Input file read: {} bytes, {} lines",
           code.len(), code.lines().count());

    debug!("📋 Pipeline Stage 4: WebGPU output generator creation");

    // Create the pure WebGPU output generator
    let mut output_generator = OutputGenerator::new(config)
        .map_err(|e| {
            warn!("❌ Failed to create WebGPU output generator: {}", e);
            anyhow::anyhow!("Failed to create WebGPU output generator: {}", e)
        })?;
    debug!("✅ WebGPU output generator created successfully");

    debug!("📋 Pipeline Stage 5: WebGPU output generation");

    // Generate gorgeous WebGPU output
    output_generator
        .generate(&code)
        .await
        .map_err(|e| {
            warn!("❌ Failed to generate WebGPU output: {}", e);
            anyhow::anyhow!("Failed to generate WebGPU output: {}", e)
        })?;

    info!("🎉 CodeSkew pipeline completed successfully!");
    debug!("📊 Pipeline execution finished - check DEBUG logs for detailed analysis");

    Ok(())
}
