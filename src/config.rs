use crate::cli::{Cli, OutputFormat};
use crate::error::CodeSkewError;
use anyhow::Result;
use std::path::{Path, PathBuf};

/// Gradient colors for the background - optimized for performance
#[derive(Debug, Clone)]
pub struct GradientColors {
    pub start: String,
    pub end: String,
}

impl GradientColors {
    /// Create a new gradient colors instance
    #[inline]
    pub fn new(start: String, end: String) -> Self {
        Self { start, end }
    }

    /// Create gradient colors from CLI arguments - optimized for minimal allocation
    #[inline]
    pub fn from_cli(cli: &Cli) -> Self {
        Self::new(cli.gradient_start.clone(), cli.gradient_end.clone())
    }

    /// Get the start color
    #[inline]
    pub fn start(&self) -> &str {
        &self.start
    }

    /// Get the end color
    #[inline]
    pub fn end(&self) -> &str {
        &self.end
    }

    /// Validate gradient colors format
    #[inline]
    pub fn validate(&self) -> Result<(), CodeSkewError> {
        // Validate start color format using the accessor method
        if !self.is_valid_color(self.start()) {
            return Err(CodeSkewError::ConfigError(format!(
                "Invalid gradient start color: {}",
                self.start()
            )));
        }

        // Validate end color format using the accessor method
        if !self.is_valid_color(self.end()) {
            return Err(CodeSkewError::ConfigError(format!(
                "Invalid gradient end color: {}",
                self.end()
            )));
        }

        Ok(())
    }

    /// Check if a color string is valid (hex format)
    #[inline]
    fn is_valid_color(&self, color: &str) -> bool {
        if color.starts_with('#') && color.len() == 7 {
            color[1..].chars().all(|c| c.is_ascii_hexdigit())
        } else {
            false
        }
    }
}

/// Configuration for the code rendering - optimized for blazing performance
#[derive(Debug, Clone)]
pub struct Config {
    pub input: PathBuf,
    pub output: PathBuf,
    pub width: u32,
    pub height: u32,
    pub font: String,
    pub fontsize: f32,
    pub skew: f32,
    pub depth: f32,
    pub perspective: f32,
    pub blur: f32,
    pub animate: bool,
    pub theme: String,
    pub centered: bool,
    pub gradient: GradientColors,
    pub format: OutputFormat,
    pub telegram: bool,
    pub duration: f32,
    pub fps: f32,
    pub shader: String,
    
    // 3D perspective parameters
    pub fold: f32,
    pub skew_angle: f32,
    pub scale: f32,
}

impl Config {
    /// Create a new configuration from CLI arguments - zero allocation optimized
    #[inline]
    pub fn from_cli(cli: &Cli) -> Result<Self> {
        // Handle --live flag as shorthand for --format wgpu
        let mut format = if cli.live {
            OutputFormat::Wgpu
        } else {
            match cli.format {
                Some(format) => format,
                None => Self::determine_format_from_extension(cli.output.as_path()),
            }
        };

        // Create the gradient colors with minimal allocation
        let gradient = GradientColors::from_cli(cli);

        // Optimize animation format selection - branch prediction friendly
        if cli.animate && format != OutputFormat::Webp && format != OutputFormat::Wgpu {
            format = OutputFormat::Gif;
        }

        // Telegram optimization - compile-time constants for dimensions
        let (width, height, telegram) = if cli.telegram {
            (512_u32, 512_u32, true)
        } else {
            (cli.width, cli.height, false)
        };

        // Create the configuration - reuse existing allocations where possible
        Ok(Config {
            input: cli.input.clone(),
            output: cli.output.clone(),
            width,
            height,
            font: cli.font.clone(),
            fontsize: cli.fontsize,
            skew: cli.skew,
            depth: cli.depth,
            perspective: cli.perspective,
            blur: cli.blur,
            animate: cli.animate || telegram, // Always animate for Telegram
            theme: cli.theme.clone(),
            centered: cli.centered,
            gradient,
            format: if telegram { OutputFormat::Webp } else { format },
            telegram,
            duration: cli.duration,
            fps: cli.fps,
            shader: cli.shader.clone(),
            
            // 3D perspective parameters
            fold: cli.fold,
            skew_angle: cli.skew_angle,
            scale: cli.scale,
        })
    }

    /// Determine output format from file extension - zero allocation approach
    #[inline]
    fn determine_format_from_extension(path: &Path) -> OutputFormat {
        // Optimized extension matching - avoid allocation by direct comparison
        match path.extension().and_then(|ext| ext.to_str()) {
            Some(ext) => match ext {
                "svg" | "SVG" => OutputFormat::Svg,
                "gif" | "GIF" => OutputFormat::Gif,
                "webp" | "WEBP" | "WebP" => OutputFormat::Webp,
                _ => OutputFormat::Png,
            },
            None => OutputFormat::Png,
        }
    }

    /// Validate the configuration - optimized error handling with early returns
    #[inline]
    pub fn validate(&self) -> Result<(), CodeSkewError> {
        // Fast path validation - check most likely failures first

        // Check dimensions first (most common validation failure)
        if self.width == 0 || self.height == 0 {
            return Err(CodeSkewError::ConfigError(format!(
                "Invalid dimensions: {}x{}",
                self.width, self.height
            )));
        }

        // Check font size (second most common validation failure)
        if self.fontsize <= 0.0 {
            return Err(CodeSkewError::ConfigError(format!(
                "Invalid font size: {}",
                self.fontsize
            )));
        }

        // Validate font name is not empty
        if self.font.is_empty() {
            return Err(CodeSkewError::ConfigError(
                "Font name cannot be empty".to_string(),
            ));
        }

        // Validate skew range for visual appeal
        if self.skew < -45.0 || self.skew > 45.0 {
            return Err(CodeSkewError::ConfigError(format!(
                "Skew angle out of range: {} (must be between -45 and 45)",
                self.skew
            )));
        }

        // Validate depth for 3D effect
        if self.depth < 0.0 {
            return Err(CodeSkewError::ConfigError(format!(
                "Depth must be non-negative: {}",
                self.depth
            )));
        }

        // Validate perspective for 3D effect
        if self.perspective < 0.0 {
            return Err(CodeSkewError::ConfigError(format!(
                "Perspective must be non-negative: {}",
                self.perspective
            )));
        }

        // Validate blur for visual quality
        if self.blur < 0.0 {
            return Err(CodeSkewError::ConfigError(format!(
                "Blur must be non-negative: {}",
                self.blur
            )));
        }

        // Check animation settings for consistency
        if self.animate && self.duration <= 0.0 {
            return Err(CodeSkewError::ConfigError(format!(
                "Animation duration must be positive when animate is enabled: {}",
                self.duration
            )));
        }

        // Validate centered flag consistency with dimensions
        if self.centered && (self.width < 100 || self.height < 100) {
            return Err(CodeSkewError::ConfigError(
                "Centered mode requires minimum dimensions of 100x100".to_string(),
            ));
        }

        // Validate gradient colors
        self.gradient.validate()?;

        // File system checks (more expensive, so done last)
        if !self.input.exists() {
            return Err(CodeSkewError::ConfigError(format!(
                "Input file not found: {}",
                self.input.display()
            )));
        }

        if !self.input.is_file() {
            return Err(CodeSkewError::ConfigError(format!(
                "Input is not a file: {}",
                self.input.display()
            )));
        }

        // Output directory validation with optimized path checking
        if let Some(parent) = self.output.parent() {
            // Optimized empty path check using as_os_str for zero allocation
            if !parent.as_os_str().is_empty() && !parent.exists() {
                return Err(CodeSkewError::ConfigError(format!(
                    "Output directory not found: {}",
                    parent.display()
                )));
            }
        }

        Ok(())
    }
}
