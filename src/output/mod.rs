//! Output module for CodeSkew rendering
//!
//! This module contains the decomposed output generation components
//! for better code organization and maintainability.

pub mod composite_renderer;
pub mod output_generator;
pub mod save_methods;

// Re-export for convenience
pub use composite_renderer::*;
pub use output_generator::*;
pub use save_methods::*;
