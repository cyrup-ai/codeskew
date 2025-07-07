use crate::config::GradientColors;
use crate::error::CodeSkewError;
use anyhow::Result;
use colorgrad::{Color, Gradient as ColorGradient, LinearGradient};
use image::{Rgb, Rgba, RgbImage, RgbaImage};
use std::any::Any;

// Re-export the Gradient trait publicly
pub use colorgrad::Gradient;

/// Custom trait for gradient providers that can render to both RGB and RGBA images
pub trait GradientProvider: Any {
    /// Generate a gradient background on an RGB image
    fn generate_rgb(&self, img: &mut RgbImage) -> Result<(), CodeSkewError>;
    
    /// Generate a gradient background on an RGBA image
    fn generate_rgba(&self, img: &mut RgbaImage) -> Result<(), CodeSkewError>;
    
    /// Get the color at a specific position (0.0 to 1.0) as RGB
    fn get_rgb_at(&self, pos: f32) -> [u8; 3];
    
    /// Get the color at a specific position (0.0 to 1.0) as RGBA
    fn get_rgba_at(&self, pos: f32) -> [u8; 4];
    
    /// Convert to Any for downcasting
    fn as_any(&self) -> &dyn Any;
}

/// Generates gradient backgrounds for the code image
#[derive(Clone)]
pub struct GradientGenerator {
    gradient: LinearGradient,
}

impl GradientGenerator {
    /// Create a new gradient generator with the specified colors
    pub fn new(gradient_colors: &GradientColors) -> Self {
        // Parse the gradient colors
        let start_color = Self::parse_hex_color(&gradient_colors.start)
            .expect("Invalid start color");
        let end_color = Self::parse_hex_color(&gradient_colors.end)
            .expect("Invalid end color");

        // Create the gradient
        let gradient = colorgrad::GradientBuilder::new()
            .colors(&[start_color, end_color])
            .build::<LinearGradient>()
            .expect("Failed to create gradient");

        Self { gradient }
    }

    /// Parse a hex color string into an RGB color
    fn parse_hex_color(hex: &str) -> Result<Color, CodeSkewError> {
        let hex = hex.trim_start_matches('#');
        
        // Parse the hex color
        let r = u8::from_str_radix(&hex[0..2], 16)
            .map_err(|_| CodeSkewError::RenderingError(format!("Invalid hex color: #{}", hex)))?;
        let g = u8::from_str_radix(&hex[2..4], 16)
            .map_err(|_| CodeSkewError::RenderingError(format!("Invalid hex color: #{}", hex)))?;
        let b = u8::from_str_radix(&hex[4..6], 16)
            .map_err(|_| CodeSkewError::RenderingError(format!("Invalid hex color: #{}", hex)))?;
        
        // Convert to 0.0-1.0 range and explicitly cast to f32
        let r = (r as f32) / 255.0;
        let g = (g as f32) / 255.0;
        let b = (b as f32) / 255.0;
        
        Ok(Color::new(r, g, b, 1.0))
    }

    /// Generate a gradient background image with the specified dimensions
    pub fn generate(&self, width: u32, height: u32) -> Result<RgbaImage> {
        // Create the image buffer
        let mut img = RgbaImage::new(width, height);
        
        // Fill the image with the gradient
        for y in 0..height {
            // Calculate the gradient position (0.0 to 1.0)
            let pos = y as f32 / height as f32;
            
            // Get the color at this position - using f32 as required by the API
            let color = self.gradient.at(pos);
            
            // Convert to RGBA u8 values
            let r = (color.r * 255.0) as u8;
            let g = (color.g * 255.0) as u8;
            let b = (color.b * 255.0) as u8;
            let a = 255; // Full opacity
            
            // Fill this row with the color
            for x in 0..width {
                img.put_pixel(x, y, Rgba([r, g, b, a]));
            }
        }
        
        Ok(img)
    }
}

// Implement the colorgrad::Gradient trait for GradientGenerator
impl ColorGradient for GradientGenerator {
    fn at(&self, t: f32) -> Color {
        self.gradient.at(t)
    }
    
    fn domain(&self) -> (f32, f32) {
        self.gradient.domain()
    }
}

// Implement our custom GradientProvider trait for GradientGenerator
impl GradientProvider for GradientGenerator {
    fn generate_rgb(&self, img: &mut RgbImage) -> Result<(), CodeSkewError> {
        let width = img.width();
        let height = img.height();
        
        // Fill the image with the gradient
        for y in 0..height {
            // Calculate the gradient position (0.0 to 1.0)
            let pos = y as f32 / height as f32;
            
            // Get the color at this position
            let color = self.gradient.at(pos);
            let r = (color.r * 255.0) as u8;
            let g = (color.g * 255.0) as u8;
            let b = (color.b * 255.0) as u8;
            
            // Fill this row with the color
            for x in 0..width {
                img.put_pixel(x, y, Rgb([r, g, b]));
            }
        }
        
        Ok(())
    }
    
    fn generate_rgba(&self, img: &mut RgbaImage) -> Result<(), CodeSkewError> {
        let width = img.width();
        let height = img.height();
        
        // Fill the image with the gradient
        for y in 0..height {
            // Calculate the gradient position (0.0 to 1.0)
            let pos = y as f32 / height as f32;
            
            // Get the color at this position
            let color = self.gradient.at(pos);
            let r = (color.r * 255.0) as u8;
            let g = (color.g * 255.0) as u8;
            let b = (color.b * 255.0) as u8;
            let a = 255; // Full opacity
            
            // Fill this row with the color
            for x in 0..width {
                img.put_pixel(x, y, Rgba([r, g, b, a]));
            }
        }
        
        Ok(())
    }
    
    fn get_rgb_at(&self, pos: f32) -> [u8; 3] {
        let color = self.gradient.at(pos);
        let r = (color.r * 255.0) as u8;
        let g = (color.g * 255.0) as u8;
        let b = (color.b * 255.0) as u8;
        [r, g, b]
    }
    
    fn get_rgba_at(&self, pos: f32) -> [u8; 4] {
        let color = self.gradient.at(pos);
        let r = (color.r * 255.0) as u8;
        let g = (color.g * 255.0) as u8;
        let b = (color.b * 255.0) as u8;
        let a = 255; // Full opacity
        [r, g, b, a]
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
}
