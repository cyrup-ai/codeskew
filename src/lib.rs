// CodeSkew: A tool for rendering code with 3D perspective effects
//
// This library provides functionality for rendering code snippets with 3D perspective effects,
// gradient backgrounds, and syntax highlighting. It supports multiple output formats including
// PNG, SVG, GIF, and WebP, with special support for Telegram sticker format.

use anyhow::Result;
use std::fs;

// Public modules
pub mod cli;
pub mod config;
pub mod error;
pub mod glyphon;
pub mod highlight;
pub mod layout;
pub mod nerdfont;
pub mod output;
pub mod shader_data;
pub mod toy;
pub mod webgpu;

// Public re-exports for main library interface
pub use cli::Cli;
pub use config::{Config, GradientColors};
pub use error::CodeSkewError;
pub use highlight::{SpanStyle, StyledLine, StyledSpan, SyntaxHighlighter};
pub use layout::{LayoutEngine, PositionedLine};
pub use output::OutputGenerator;
pub use toy::*;
// webgpu utilities available if needed

/// Process a code file with gorgeous WebGPU rendering
///
/// This is the main entry point for pure WebGPU rendering.
///
/// # Arguments
///
/// * `config` - The configuration for rendering the code
/// * `code` - The source code to render
///
/// # Returns
///
/// * `Result<()>` - Ok if the processing was successful, Err otherwise
pub async fn process(config: Config, code: &str) -> Result<()> {
    // Create the pure WebGPU output generator
    let mut output_generator = OutputGenerator::new(config)
        .map_err(|e| anyhow::anyhow!("Failed to create WebGPU output generator: {}", e))?;

    // Generate gorgeous WebGPU output
    output_generator
        .generate(code)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to generate WebGPU output: {}", e))?;

    Ok(())
}

/// Process a code file from a file path with gorgeous WebGPU rendering
///
/// # Arguments
///
/// * `config` - The configuration for rendering the code
///
/// # Returns
///
/// * `Result<()>` - Ok if the processing was successful, Err otherwise
pub async fn process_file(config: Config) -> Result<()> {
    // Read the input file
    let code = fs::read_to_string(&config.input)?;

    // Process the code with WebGPU
    process(config, &code).await
}
