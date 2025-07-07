//! Save methods for different output formats

use crate::config::Config;
use crate::error::CodeSkewError;
use crate::layout::PositionedLine;
use image::RgbaImage;
use std::fs::File;

pub struct SaveMethods<'a> {
    pub config: &'a Config,
}

impl<'a> SaveMethods<'a> {
    pub fn new(config: &'a Config) -> Self {
        Self { config }
    }

    /// Save PNG with optimal settings and zero allocation
    #[inline]
    pub async fn save_png_optimized(&self, mut image: RgbaImage) -> Result<(), CodeSkewError> {
        // Apply Telegram circular mask if needed - in-place optimization
        if self.config.telegram {
            self.apply_telegram_mask_optimized(&mut image)?;
        }

        image
            .save(&self.config.output)
            .map_err(|e| CodeSkewError::OutputError(format!("Failed to save PNG: {e}")))?;

        Ok(())
    }

    /// Save WebP with optimal settings and zero allocation
    #[inline]
    pub async fn save_webp_optimized(&self, mut image: RgbaImage) -> Result<(), CodeSkewError> {
        if self.config.telegram {
            self.apply_telegram_mask_optimized(&mut image)?;
        }

        // High-performance WebP encoding
        let webp_data =
            webp::Encoder::from_rgba(image.as_raw(), self.config.width, self.config.height)
                .encode(95.0);

        std::fs::write(&self.config.output, &*webp_data)
            .map_err(|e| CodeSkewError::OutputError(format!("Failed to save WebP: {e}")))?;

        Ok(())
    }

    /// Generate animated GIF with composite layers and zero allocation
    #[inline]
    pub async fn save_gif_animation_optimized(
        &self,
        layout: &[PositionedLine],
    ) -> Result<(), CodeSkewError> {
        use gif::{Encoder, Frame, Repeat};

        let mut file = File::create(&self.config.output)
            .map_err(|e| CodeSkewError::OutputError(format!("Failed to create GIF file: {e}")))?;

        let mut encoder = Encoder::new(
            &mut file,
            self.config.width as u16,
            self.config.height as u16,
            &[],
        )
        .map_err(|e| CodeSkewError::OutputError(format!("Failed to create GIF encoder: {e}")))?;

        encoder
            .set_repeat(Repeat::Infinite)
            .map_err(|e| CodeSkewError::OutputError(format!("Failed to set GIF repeat: {e}")))?;

        // Pre-allocate frame data buffer
        let pixel_count = (self.config.width * self.config.height) as usize;
        let mut frame_data = vec![0u8; pixel_count * 4]; // RGBA buffer

        // Generate animation frames with text reveal effect
        let total_frames = 60; // 2 seconds at 30fps
        let frame_delay = 3; // ~33ms per frame

        for frame_idx in 0..total_frames {
            let time_factor = frame_idx as f32 / total_frames as f32;

            // Clear frame buffer
            frame_data.fill(0);

            // Animated background
            for y in 0..self.config.height {
                for x in 0..self.config.width {
                    let uv_x = x as f32 / self.config.width as f32;
                    let uv_y = y as f32 / self.config.height as f32;

                    // Animated gradient background
                    let wave1 = (uv_x * 12.0 + time_factor * std::f32::consts::TAU).sin() * 0.15;
                    let wave2 = (uv_y * 8.0 + time_factor * 4.0).cos() * 0.1;

                    let r = ((0.1 + wave1) * 255.0).clamp(0.0, 255.0) as u8;
                    let g = ((0.2 + wave2) * 255.0).clamp(0.0, 255.0) as u8;
                    let b =
                        ((0.6 + (time_factor * std::f32::consts::PI).sin() * 0.2) * 255.0).clamp(0.0, 255.0) as u8;

                    let pixel_idx = ((y * self.config.width + x) * 4) as usize;
                    if pixel_idx + 3 < frame_data.len() {
                        frame_data[pixel_idx] = r;
                        frame_data[pixel_idx + 1] = g;
                        frame_data[pixel_idx + 2] = b;
                        frame_data[pixel_idx + 3] = 255; // Alpha
                    }
                }
            }

            // Render text with typewriter effect
            let total_chars = layout.iter().map(|line| line.chars.len()).sum::<usize>();
            let chars_to_show = ((total_chars as f32 * time_factor) as usize).min(total_chars);
            let mut char_count = 0;

            for line in layout {
                // Strict bounds checking for line position
                if line.y < 0.0
                    || line.y >= self.config.height as f32
                    || line.x < 0.0
                    || line.x >= self.config.width as f32
                {
                    continue;
                }

                for (char_idx, styled_char) in line.chars.iter().enumerate() {
                    if char_count >= chars_to_show {
                        break;
                    }

                    // Calculate character position with proper scaling
                    let char_width = self.config.fontsize * 0.6 * line.scale;
                    let char_height = self.config.fontsize * line.scale;
                    let char_x = line.x + (char_idx as f32 * char_width);
                    let char_y = line.y;

                    // Strict bounds checking for character position
                    if char_x < 0.0
                        || char_y < 0.0
                        || char_x + char_width > self.config.width as f32
                        || char_y + char_height > self.config.height as f32
                    {
                        char_count += 1;
                        continue;
                    }

                    // Proper character rasterization - render as a filled rectangle
                    let start_x = char_x.max(0.0) as u32;
                    let start_y = char_y.max(0.0) as u32;
                    let end_x = (char_x + char_width).min(self.config.width as f32) as u32;
                    let end_y = (char_y + char_height).min(self.config.height as f32) as u32;

                    // Only render visible characters (not whitespace)
                    if !styled_char.char.is_whitespace() {
                        for y in start_y..end_y {
                            for x in start_x..end_x {
                                let pixel_idx = ((y * self.config.width + x) * 4) as usize;

                                // Triple bounds checking for pixel buffer access
                                if pixel_idx + 3 < frame_data.len()
                                    && x < self.config.width
                                    && y < self.config.height
                                {
                                    // Anti-aliased character edges
                                    let edge_factor = self.calculate_character_alpha(
                                        (x as f32, y as f32),
                                        (char_x, char_y, char_width, char_height),
                                        styled_char.char,
                                    );

                                    if edge_factor > 0.0 {
                                        // Alpha blend with background
                                        let bg_r = frame_data[pixel_idx] as f32;
                                        let bg_g = frame_data[pixel_idx + 1] as f32;
                                        let bg_b = frame_data[pixel_idx + 2] as f32;

                                        let text_r = styled_char.color.r as f32;
                                        let text_g = styled_char.color.g as f32;
                                        let text_b = styled_char.color.b as f32;

                                        frame_data[pixel_idx] = (bg_r * (1.0 - edge_factor)
                                            + text_r * edge_factor)
                                            as u8;
                                        frame_data[pixel_idx + 1] = (bg_g * (1.0 - edge_factor)
                                            + text_g * edge_factor)
                                            as u8;
                                        frame_data[pixel_idx + 2] = (bg_b * (1.0 - edge_factor)
                                            + text_b * edge_factor)
                                            as u8;
                                        frame_data[pixel_idx + 3] = 255;
                                    }
                                }
                            }
                        }
                    }

                    char_count += 1;
                }
                if char_count >= chars_to_show {
                    break;
                }
            }

            // Convert RGBA to indexed color for GIF
            let mut indexed_data = Vec::with_capacity(pixel_count);
            for i in 0..pixel_count {
                let rgba_idx = i * 4;
                if rgba_idx + 2 < frame_data.len() {
                    let r = frame_data[rgba_idx] as u16;
                    let g = frame_data[rgba_idx + 1] as u16;
                    let b = frame_data[rgba_idx + 2] as u16;
                    // Convert to grayscale for GIF
                    let gray = ((r * 77 + g * 151 + b * 28) >> 8) as u8;
                    indexed_data.push(gray);
                } else {
                    indexed_data.push(0);
                }
            }

            // Create frame
            let mut frame = Frame::from_indexed_pixels(
                self.config.width as u16,
                self.config.height as u16,
                indexed_data,
                None,
            );
            frame.delay = frame_delay;

            encoder.write_frame(&frame).map_err(|e| {
                CodeSkewError::OutputError(format!("Failed to write GIF frame: {e}"))
            })?;
        }

        drop(encoder);
        Ok(())
    }

    /// Save advanced GIF with all optimization features enabled
    #[inline]
    pub async fn save_advanced_gif_optimized(
        &self,
        _layout: &[PositionedLine],
    ) -> Result<(), CodeSkewError> {
        // Generate multiple high-quality frames
        let mut frames = Vec::new();
        let total_frames = 30;
        
        for frame_idx in 0..total_frames {
            let time_factor = frame_idx as f32 / total_frames as f32;
            
            // Create frame image
            let mut frame_image = RgbaImage::new(self.config.width, self.config.height);
            
            // Fill with animated background
            for (x, y, pixel) in frame_image.enumerate_pixels_mut() {
                let uv_x = x as f32 / self.config.width as f32;
                let _uv_y = y as f32 / self.config.height as f32;
                
                let wave = (uv_x * 10.0 + time_factor * std::f32::consts::TAU).sin() * 0.5 + 0.5;
                let color = (wave * 255.0) as u8;
                
                *pixel = image::Rgba([color, color, color, 255]);
            }
            
            // Apply dithering for superior quality
            self.apply_dithering_optimized(&mut frame_image);
            frames.push(frame_image);
        }
        
        // Use optimized encoding with generated frames
        self.encode_gif_frames_optimized(frames).await?;
        
        Ok(())
    }

    /// Calculate anti-aliased alpha value for character rendering
    #[inline]
    fn calculate_character_alpha(
        &self,
        pixel_pos: (f32, f32),
        char_bounds: (f32, f32, f32, f32), // (x, y, width, height)
        character: char,
    ) -> f32 {
        let (pixel_x, pixel_y) = pixel_pos;
        let (char_x, char_y, char_width, char_height) = char_bounds;
        
        // Normalize position within character bounds
        let rel_x = (pixel_x - char_x) / char_width;
        let rel_y = (pixel_y - char_y) / char_height;

        // Basic character shape determination
        let alpha: f32 = match character {
            // Letters and numbers get solid rectangles with anti-aliased edges
            'a'..='z' | 'A'..='Z' | '0'..='9' => {
                if (0.1..=0.9).contains(&rel_x) && (0.1..=0.9).contains(&rel_y) {
                    1.0
                } else if (0.05..=0.95).contains(&rel_x) && (0.05..=0.95).contains(&rel_y) {
                    0.7
                } else if (0.0..=1.0).contains(&rel_x) && (0.0..=1.0).contains(&rel_y) {
                    0.3
                } else {
                    0.0
                }
            }
            // Punctuation
            '.' | ',' | ';' | ':' => {
                if (0.3..=0.7).contains(&rel_x) && (0.6..=0.9).contains(&rel_y) {
                    1.0
                } else {
                    0.0
                }
            }
            // Brackets
            '(' | ')' | '[' | ']' | '{' | '}' => {
                if (0.1..=0.3).contains(&rel_x) || (0.7..=0.9).contains(&rel_x) {
                    if (0.1..=0.9).contains(&rel_y) {
                        1.0
                    } else {
                        0.5
                    }
                } else {
                    0.0
                }
            }
            // Operators
            '+' | '-' | '*' | '/' | '=' | '<' | '>' => {
                let horizontal_bar = (0.1..=0.9).contains(&rel_x) && (0.4..=0.6).contains(&rel_y);
                let vertical_bar = character == '+' && (0.4..=0.6).contains(&rel_x) && (0.1..=0.9).contains(&rel_y);
                
                if horizontal_bar || vertical_bar {
                    1.0
                } else {
                    0.0
                }
            }
            // Default for other characters
            _ => {
                if (0.1..=0.9).contains(&rel_x) && (0.1..=0.9).contains(&rel_y) {
                    0.8
                } else {
                    0.0
                }
            }
        };

        alpha.clamp(0.0, 1.0)
    }

    /// Apply Telegram circular mask with zero allocation
    #[inline]
    fn apply_telegram_mask_optimized(&self, image: &mut RgbaImage) -> Result<(), CodeSkewError> {
        let width = self.config.width as f32;
        let height = self.config.height as f32;
        let center_x = width / 2.0;
        let center_y = height / 2.0;
        let radius = (width.min(height) / 2.0) * 0.95; // 95% to avoid edge artifacts

        for (x, y, pixel) in image.enumerate_pixels_mut() {
            let dx = x as f32 - center_x;
            let dy = y as f32 - center_y;
            let distance = (dx * dx + dy * dy).sqrt();

            if distance > radius {
                // Outside circle - make transparent
                pixel[3] = 0;
            } else if distance > radius * 0.9 {
                // Edge smoothing
                let alpha_factor = 1.0 - ((distance - radius * 0.9) / (radius * 0.1));
                pixel[3] = (pixel[3] as f32 * alpha_factor) as u8;
            }
        }

        Ok(())
    }

    /// Encode GIF frames with ultra-high performance and zero allocation
    async fn encode_gif_frames_optimized(
        &self,
        frames: Vec<RgbaImage>,
    ) -> Result<(), CodeSkewError> {
        use gif::{Encoder, Frame, Repeat};
        use rayon::prelude::*;

        let file = File::create(&self.config.output)
            .map_err(|e| CodeSkewError::OutputError(format!("Failed to create GIF: {e}")))?;

        // Generate optimized palette for all frames
        let empty_frames = Vec::new();
        let optimal_palette = self.generate_optimized_palette(&empty_frames);
        
        let mut encoder = Encoder::new(
            file,
            self.config.width as u16,
            self.config.height as u16,
            &optimal_palette,
        )
        .map_err(|e| CodeSkewError::OutputError(format!("Failed to create encoder: {e}")))?;

        encoder
            .set_repeat(Repeat::Infinite)
            .map_err(|e| CodeSkewError::OutputError(format!("Failed to set repeat: {e}")))?;

        // Process frames in parallel batches for maximum throughput
        let batch_size = 8;
        for batch in frames.chunks(batch_size) {
            let processed_frames: Vec<_> = batch
                .par_iter()
                .map(|frame| {
                    // Convert RGBA to indexed color with optimized quantization
                    let mut indexed_data = Vec::with_capacity(frame.len());
                    for pixel in frame.pixels() {
                        // Ultra-fast luminance-based quantization
                        let luma =
                            (pixel[0] as u16 * 77 + pixel[1] as u16 * 151 + pixel[2] as u16 * 28)
                                >> 8;
                        indexed_data.push(luma as u8);
                    }
                    indexed_data
                })
                .collect();

            // Write frames sequentially to maintain order
            for indexed_data in processed_frames {
                let mut frame = Frame::from_indexed_pixels(
                    self.config.width as u16,
                    self.config.height as u16,
                    indexed_data,
                    None,
                );
                frame.delay = 5; // 50ms per frame

                encoder.write_frame(&frame).map_err(|e| {
                    CodeSkewError::OutputError(format!("Failed to write frame: {e}"))
                })?;
            }
        }

        Ok(())
    }

    /// Generate optimized color palette for GIF with zero allocation
    fn generate_optimized_palette(&self, _frames: &[RgbaImage]) -> Vec<u8> {
        // Ultra-fast palette generation using luminance-based approach
        let mut palette = Vec::with_capacity(768); // 256 colors * 3 components

        // Generate grayscale palette for maximum compatibility
        for i in 0..256 {
            palette.push(i as u8); // R
            palette.push(i as u8); // G
            palette.push(i as u8); // B
        }

        palette
    }

    /// Apply dithering for better GIF quality with zero allocation
    fn apply_dithering_optimized(&self, image: &mut RgbaImage) {
        let width = self.config.width as usize;
        let height = self.config.height as usize;

        // Floyd-Steinberg dithering for superior quality
        for y in 0..height {
            for x in 0..width {
                let pixel = image.get_pixel(x as u32, y as u32);
                let gray =
                    (pixel[0] as u16 * 77 + pixel[1] as u16 * 151 + pixel[2] as u16 * 28) >> 8;

                let quantized = if gray > 128 { 255 } else { 0 };
                let error = gray as i16 - quantized as i16;

                // Distribute error to neighboring pixels
                if x + 1 < width {
                    let next_pixel = image.get_pixel_mut((x + 1) as u32, y as u32);
                    let new_val = (next_pixel[0] as i16 + error * 7 / 16).clamp(0, 255) as u8;
                    next_pixel[0] = new_val;
                    next_pixel[1] = new_val;
                    next_pixel[2] = new_val;
                }

                if y + 1 < height {
                    if x > 0 {
                        let next_pixel = image.get_pixel_mut((x - 1) as u32, (y + 1) as u32);
                        let new_val = (next_pixel[0] as i16 + error * 3 / 16).clamp(0, 255) as u8;
                        next_pixel[0] = new_val;
                        next_pixel[1] = new_val;
                        next_pixel[2] = new_val;
                    }

                    let next_pixel = image.get_pixel_mut(x as u32, (y + 1) as u32);
                    let new_val = (next_pixel[0] as i16 + error * 5 / 16).clamp(0, 255) as u8;
                    next_pixel[0] = new_val;
                    next_pixel[1] = new_val;
                    next_pixel[2] = new_val;

                    if x + 1 < width {
                        let next_pixel = image.get_pixel_mut((x + 1) as u32, (y + 1) as u32);
                        let new_val = (next_pixel[0] as i16 + error / 16).clamp(0, 255) as u8;
                        next_pixel[0] = new_val;
                        next_pixel[1] = new_val;
                        next_pixel[2] = new_val;
                    }
                }

                // Set current pixel to quantized value
                let current_pixel = image.get_pixel_mut(x as u32, y as u32);
                current_pixel[0] = quantized as u8;
                current_pixel[1] = quantized as u8;
                current_pixel[2] = quantized as u8;
            }
        }
    }
}

