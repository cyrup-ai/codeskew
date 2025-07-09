use clap::{Parser, ValueEnum};
use std::fmt;
use std::path::PathBuf;

/// CLI arguments for the codeskew tool
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Input code file to render
    #[arg(required = true)]
    pub input: PathBuf,

    /// Output image file
    #[arg(short, long, default_value = "output.png")]
    pub output: PathBuf,

    /// Output format
    #[arg(short = 'F', long)]
    pub format: Option<OutputFormat>,

    /// Width of the output image
    #[arg(short, long, default_value_t = 1200)]
    pub width: u32,

    /// Height of the output image
    #[arg(short = 'H', long, default_value_t = 800)]
    pub height: u32,

    /// Font to use for rendering
    #[arg(short, long, default_value = "FiraCode Nerd Font Mono")]
    pub font: String,

    /// Font size
    #[arg(short = 's', long, default_value_t = 14.0)]
    pub fontsize: f32,

    /// Enable ligatures (auto-detects based on font)
    #[arg(long, default_value_t = true)]
    pub ligatures: bool,

    /// Enable programming ligatures (=>, !=, etc.)
    #[arg(long, default_value_t = true)]
    pub programming_ligatures: bool,

    /// Enable typography ligatures (fi, fl, etc.)
    #[arg(long, default_value_t = false)]
    pub typography_ligatures: bool,

    /// Ligature configuration file (YAML)
    #[arg(long)]
    pub ligature_config: Option<PathBuf>,

    /// Skew angle in degrees
    #[arg(short = 'k', long, default_value_t = 15.0)]
    pub skew: f32,

    /// Depth factor for 3D effect
    #[arg(short, long, default_value_t = 0.5)]
    pub depth: f32,

    /// Perspective distance for 3D effect
    #[arg(short = 'p', long, default_value_t = 1000.0)]
    pub perspective: f32,

    /// Apply blur effect (blur amount)
    #[arg(short, long, default_value_t = 0.0)]
    pub blur: f32,

    /// Start gradient color (hex)
    #[arg(long, default_value = "#1a1a1a")]
    pub gradient_start: String,

    /// End gradient color (hex)
    #[arg(long, default_value = "#4a4a4a")]
    pub gradient_end: String,

    /// Enable animation (for GIF output)
    #[arg(short, long, default_value_t = false)]
    pub animate: bool,

    /// Center the code in the image
    #[arg(short = 'c', long, default_value_t = false)]
    pub centered: bool,

    /// Syntax highlighting theme
    #[arg(short = 't', long, default_value = "monokai")]
    pub theme: String,

    /// Create Telegram-compatible sticker (512x512 round WebP)
    #[arg(short = 'T', long, default_value_t = false)]
    pub telegram: bool,

    /// Animation duration in seconds (for animated formats)
    #[arg(long, default_value_t = 3.0)]
    pub duration: f32,

    /// Animation frames per second (for animated formats)
    #[arg(long, default_value_t = 30.0)]
    pub fps: f32,

    /// Background shader for the composite renderer
    #[arg(long, default_value = "bandwidth")]
    pub shader: String,

    /// Launch live animated preview window (shorthand for --format wgpu)
    #[arg(short = 'L', long, default_value_t = false)]
    pub live: bool,

    /// 3D fold strength (vertical scaling at 2/3 point)
    #[arg(long, default_value_t = 0.4)]
    pub fold: f32,

    /// Text skew angle for 3D perspective effect
    #[arg(long, default_value_t = 0.15)]
    pub skew_angle: f32,

    /// Overall 3D scale factor for perspective effects
    #[arg(long, default_value_t = 0.6)]
    pub scale: f32,
}

/// Output format for the rendered code
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum, Default)]
pub enum OutputFormat {
    /// PNG image format
    #[default]
    Png,
    /// SVG vector format
    Svg,
    /// Animated GIF format
    Gif,
    /// Animated WebP format
    Webp,
    /// Live animated WGPU window
    Wgpu,
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutputFormat::Png => write!(f, "png"),
            OutputFormat::Svg => write!(f, "svg"),
            OutputFormat::Gif => write!(f, "gif"),
            OutputFormat::Webp => write!(f, "webp"),
            OutputFormat::Wgpu => write!(f, "wgpu"),
        }
    }
}
