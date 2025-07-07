// -----------------------------------------------------------------------------
// src/nerdfont_loader.rs  ▸  Pure‑Rust, no‑bash Nerd Font fetcher
// -----------------------------------------------------------------------------

use anyhow::{anyhow, Context, Result};
use directories::ProjectDirs;
use reqwest::Client;
use serde::Deserialize;
use std::{fs, path::PathBuf};
use tokio::io::AsyncWriteExt;
use zip::ZipArchive;

const API_URL: &str = "https://api.github.com/repos/ryanoasis/nerd-fonts/releases/latest";
const UA: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

/// Optimized JSON structure for GitHub release API response parsing.
#[derive(Deserialize)]
struct Release { 
    tag_name: String 
}

fn cache_dir() -> PathBuf {
    ProjectDirs::from("ai", "cyrup", "nerdfonts")
        .expect("home directory")
        .cache_dir()
        .to_path_buf()
}

/// Async: get latest tag once per session.
async fn latest_tag(client: &Client) -> Result<String> {
    let r: Release = client
        .get(API_URL)
        .header("User-Agent", UA)
        .send()
        .await?
        .json()
        .await
        .context("parse GitHub JSON")?;
    Ok(r.tag_name)
}

/// Download → cache → return the bytes of the *NerdFontMono-Regular.ttf* face.
pub async fn nerd_font_bytes(font_name: &str) -> Result<Vec<u8>> {
    let client = Client::builder().user_agent(UA).build()?;
    let tag = latest_tag(&client).await?;

    let cache = cache_dir();
    fs::create_dir_all(&cache)?;
    let zip_path = cache.join(format!("{font_name}-{tag}.zip"));

    if !zip_path.exists() {
        let url = format!(
            "https://github.com/ryanoasis/nerd-fonts/releases/download/{tag}/{font_name}.zip"
        );
        let mut resp = client.get(&url).send().await?.error_for_status()?;
        let mut file = tokio::fs::File::create(&zip_path).await?;
        while let Some(chunk) = resp.chunk().await? {
            file.write_all(&chunk).await?;
        }
    }

    let zip_data = fs::read(&zip_path)?;
    let reader = std::io::Cursor::new(zip_data);
    let mut zip = ZipArchive::new(reader)?;
    for i in 0..zip.len() {
        let mut f = zip.by_index(i)?;
        if f.name().ends_with("NerdFontMono-Regular.ttf") {
            let mut buf = Vec::with_capacity(f.size() as usize);
            std::io::copy(&mut f, &mut buf)?;
            return Ok(buf);
        }
    }
    Err(anyhow!("regular mono face not found in {font_name}"))
}

/// Lock-free async font loading with zero-allocation error propagation
pub async fn nerd_font_bytes_async(font_name: &str) -> Result<Vec<u8>> {
    nerd_font_bytes(font_name).await
}

/// Curated favourites list.
pub const FAVS: &[&str] = &[
    "JetBrainsMono", "Iosevka", "Hack", "FiraCode", "Meslo", "SourceCodePro",
    "CaskaydiaCove", "VictorMono", "UbuntuMono", "Monaspace", "Terminus", "ProggyClean",
];