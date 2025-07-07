//! Production-quality Glyphon text rendering
//! 
//! This module contains ratagpu's battle-tested, zero-allocation Glyphon
//! text rendering system adapted for hybrid render-to-texture usage.

pub mod cache;
pub mod cell;
pub mod color;
pub mod font_system;
pub mod text_rendering;
pub mod texture_renderer;
pub mod wgpu_setup;

// Re-export key types
pub use cache::{ZeroAllocTextAreaPool, LockFreeShapeCache, SafeTextArea};
pub use cell::{Cell, CellGrid};
pub use color::ColorPalette;
pub use font_system::{create_optimized_font_system, FontMetrics};
pub use text_rendering::{ZeroAllocTextRenderer, TextRenderConfig};
pub use texture_renderer::GlyphonTextureRenderer;