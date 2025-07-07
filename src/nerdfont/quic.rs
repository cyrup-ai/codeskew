// -----------------------------------------------------------------------------
// src/quic_fontloader.rs  ▸  "QUIC" 🍃 super‑light font loader for RataGPU
// -----------------------------------------------------------------------------
#![allow(dead_code)]

use std::path::PathBuf;
use thiserror::Error;

/// Supported Nerd Font families
#[derive(Debug, Clone, Copy)]
pub enum NerdFont {
    Iosevka,
    FiraCodeNerdFontMono,
    JetBrainsMono,
    CascaydiaCove,
    Hack,
}

impl NerdFont {
    /// Get the GitHub download name for this font
    pub fn github_name(&self) -> &'static str {
        match self {
            Self::Iosevka => "Iosevka",
            Self::FiraCodeNerdFontMono => "FiraCode",
            Self::JetBrainsMono => "JetBrainsMono", 
            Self::CascaydiaCove => "CascaydiaCove",
            Self::Hack => "Hack",
        }
    }
}

impl From<&str> for NerdFont {
    fn from(name: &str) -> Self {
        match name {
            "Iosevka" => Self::Iosevka,
            "FiraCode" => Self::FiraCodeNerdFontMono,
            "FiraCode Nerd Font Mono" => Self::FiraCodeNerdFontMono,
            "JetBrains" => Self::JetBrainsMono,
            "JetBrainsMono" => Self::JetBrainsMono,
            "Cascadia" => Self::CascaydiaCove,
            "CascaydiaCove" => Self::CascaydiaCove,
            "SF" => Self::Hack,
            "Hack" => Self::Hack,
            _ => Self::FiraCodeNerdFontMono, // Default to FiraCode Nerd Font Mono
        }
    }
}

#[cfg(feature = "zstd")] 
use zstd::decode_all;

#[cfg(feature = "zstd")] 
const EMBEDDED_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/iosevka.zst"));

#[cfg(not(feature = "zstd"))] 
const EMBEDDED_BYTES: &[u8] = &[];

/// Errors that may occur while loading fonts.
#[derive(Debug, Error)]
pub enum FontLoadError {
    #[error("env‑var override {0} is not valid UTF‑8")]
    InvalidUtf8(&'static str),
    
    #[error("could not read font file {0}: {1}")]
    Io(PathBuf, #[source] std::io::Error),
    
    #[cfg(feature = "zstd")]
    #[error("zstd decompression failed: {0}")]
    Zstd(#[from] std::io::Error),
}

/// Loads fonts into an existing `glyphon::FontSystem`.
pub async fn load_into(font_system: &mut glyphon::FontSystem) -> Result<(), FontLoadError> {
    load_with_family(font_system, None).await
}

/// Loads fonts with a specific font family selection asynchronously.
pub async fn load_with_family_async(font_system: &mut glyphon::FontSystem, font_family: Option<&str>) -> Result<(), FontLoadError> {
    let family = font_family.unwrap_or("FiraCode Nerd Font Mono");
    let nerd_font = NerdFont::from(family);
    let github_name = nerd_font.github_name();

    // Try to download font from GitHub using nerdfont_loader
    match super::loader::nerd_font_bytes(github_name).await {
        Ok(font_bytes) => {
            font_system.db_mut().load_font_data(font_bytes);
            log::info!("Downloaded and loaded {github_name} Nerd Font from GitHub");
            Ok(())
        }
        Err(e) => {
            log::warn!("Failed to download {github_name} from GitHub: {e}, using embedded fallback");
            
            // Fallback to embedded font
            #[cfg(feature = "zstd")]
            {
                let bytes = decode_all(&*EMBEDDED_BYTES)?;
                font_system.db_mut().load_font_data(bytes);
            }
            #[cfg(not(feature = "zstd"))]
            {
                font_system.db_mut().load_font_data(EMBEDDED_BYTES.to_vec());
            }
            
            log::debug!("Loaded embedded fallback font ({} KiB)", EMBEDDED_BYTES.len() / 1024);
            Ok(())
        }
    }
}

/// Loads fonts with a specific font family selection (async version).
pub async fn load_with_family(font_system: &mut glyphon::FontSystem, font_family: Option<&str>) -> Result<(), FontLoadError> {
    let family = font_family.unwrap_or("FiraCode Nerd Font Mono");
    let nerd_font = NerdFont::from(family);
    let github_name = nerd_font.github_name();

    // Try to download font from GitHub using nerdfont_loader (async)
    match super::loader::nerd_font_bytes_async(github_name).await {
        Ok(font_bytes) => {
            font_system.db_mut().load_font_data(font_bytes);
            log::info!("Downloaded and loaded {github_name} Nerd Font from GitHub");
            Ok(())
        }
        Err(e) => {
            log::warn!("Failed to download {github_name} from GitHub: {e}, using embedded fallback");
            
            // Fallback to embedded font
            #[cfg(feature = "zstd")]
            {
                let bytes = decode_all(&*EMBEDDED_BYTES)?;
                font_system.db_mut().load_font_data(bytes);
            }
            #[cfg(not(feature = "zstd"))]
            {
                font_system.db_mut().load_font_data(EMBEDDED_BYTES.to_vec());
            }
            
            log::debug!("Loaded embedded fallback font ({} KiB)", EMBEDDED_BYTES.len() / 1024);
            Ok(())
        }
    }
}

/// Load arbitrary font bytes into a `FontSystem`.
pub fn load_font_bytes(font_system: &mut glyphon::FontSystem, bytes: &[u8]) {
    font_system.db_mut().load_font_data(bytes.to_vec());
}

pub fn suggest_system_monospace() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        ["/System/Library/Fonts/Monaco.ttf", "/System/Library/Fonts/Menlo.ttc"]
            .into_iter()
            .map(PathBuf::from)
            .find(|p| p.exists())
    }
    
    #[cfg(target_os = "windows")]
    {
        ["C:/Windows/Fonts/consola.ttf", "C:/Windows/Fonts/lucon.ttf"]
            .into_iter()
            .map(PathBuf::from)
            .find(|p| p.exists())
    }
    
    #[cfg(target_os = "linux")]
    {
        [
            "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
            "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
        ]
        .into_iter()
        .map(PathBuf::from)
        .find(|p| p.exists())
    }
    
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        None
    }
}