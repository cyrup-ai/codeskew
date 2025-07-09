//! WebGPU rendering module for CodeSkew
//!
//! This module contains utilities for WebGPU operations

pub mod command_state;
pub mod text_state;
pub mod uniforms;

// Re-export for convenience
pub use command_state::CommandBufferState;
pub use text_state::{TextAreaData, TextRenderState};
pub use uniforms::ShaderUniforms;
