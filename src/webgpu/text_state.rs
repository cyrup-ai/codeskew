//! Text rendering state management for WebGPU rendering

use glyphon::{Buffer, Color, FontSystem, Metrics, TextArea, TextBounds};

/// Text rendering state for zero-copy operations
pub struct TextRenderState {
    pub buffers: Vec<Buffer>,
    pub strings: Vec<String>,
    pub areas_data: Vec<TextAreaData>,
}

/// Data for a text area without lifetime issues
#[derive(Debug, Clone)]
pub struct TextAreaData {
    pub buffer_index: usize,
    pub left: f32,
    pub top: f32,
    pub scale: f32,
    pub bounds: TextBounds,
    pub default_color: Color,
}

impl TextRenderState {
    #[inline]
    pub fn new(capacity: usize, font_system: &mut FontSystem, width: u32, height: u32, supersampling_factor: f32, base_font_size: f32) -> Self {
        let mut buffers = Vec::with_capacity(capacity);
        // Use base font size from config for buffer metrics
        let font_size = base_font_size;
        let line_height = base_font_size * 1.4; // Same ratio as layout.rs
        let buffer_width = width as f32;
        let buffer_height = height as f32;
        
        for _ in 0..capacity {
            let mut buffer = Buffer::new(font_system, Metrics::new(font_size, line_height));
            // Set buffer size to supersampled dimensions
            buffer.set_size(font_system, Some(buffer_width), Some(buffer_height));
            buffers.push(buffer);
        }

        Self {
            buffers,
            strings: Vec::with_capacity(capacity),
            areas_data: Vec::with_capacity(capacity),
        }
    }

    #[inline]
    pub fn prepare_areas(&self) -> Vec<TextArea> {
        self.areas_data
            .iter()
            .filter_map(|area_data| {
                if area_data.buffer_index < self.buffers.len() {
                    Some(TextArea {
                        buffer: &self.buffers[area_data.buffer_index],
                        left: area_data.left,
                        top: area_data.top,
                        scale: area_data.scale,
                        bounds: area_data.bounds,
                        default_color: area_data.default_color,
                        custom_glyphs: &[],
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    #[inline]
    pub fn ensure_capacity(&mut self, required: usize, font_system: &mut FontSystem, width: u32, height: u32, supersampling_factor: f32, base_font_size: f32) {
        if required > self.buffers.len() {
            self.buffers.reserve(required - self.buffers.len());
            self.strings.reserve(required - self.strings.len());
            self.areas_data.reserve(required - self.areas_data.len());
            
            // Use base font size from config for buffer metrics
            let font_size = base_font_size;
            let line_height = base_font_size * 1.4; // Same ratio as layout.rs
            let buffer_width = width as f32;
            let buffer_height = height as f32;

            while self.buffers.len() < required {
                let mut buffer = Buffer::new(font_system, Metrics::new(font_size, line_height));
                // Set buffer size to supersampled dimensions
                buffer.set_size(font_system, Some(buffer_width), Some(buffer_height));
                self.buffers.push(buffer);
            }
        }
    }
}
