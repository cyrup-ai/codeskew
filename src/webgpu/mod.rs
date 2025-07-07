//! WebGPU rendering module for CodeSkew
//!
//! This module contains the decomposed WebGPU renderer components
//! for better code organization and maintainability.

pub mod command_state;
pub mod elite_init;
pub mod elite_renderer;
pub mod text_state;
pub mod uniforms;

// Re-export for convenience
pub use command_state::CommandBufferState;
pub use elite_init::EliteWebGPURenderer;
pub use text_state::{TextAreaData, TextRenderState};
pub use uniforms::ShaderUniforms;
