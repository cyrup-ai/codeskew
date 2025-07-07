use crate::error::CodeSkewError;
use anyhow::Result;
use image::{Rgba, RgbaImage};

/// Handles perspective transformation for text rendering
pub struct PerspectiveTransformer {
    pub skew: f32,
    pub depth: f32,
    pub width: u32,
    pub height: u32,
    rotation: f32,
}

impl PerspectiveTransformer {
    /// Create a new perspective transformer with the specified parameters
    pub fn new(skew: f32, depth: f32, width: u32, height: u32) -> Self {
        Self {
            skew,
            depth,
            width,
            height,
            rotation: 0.0,
        }
    }
    
    /// Create a new perspective transformer with rotation
    pub fn new_with_rotation(
        skew: f32,
        depth: f32,
        width: u32,
        height: u32,
        rotation: f32,
    ) -> Self {
        Self {
            skew,
            depth,
            width,
            height,
            rotation,
        }
    }
    
    /// Get the transformation matrix for the specified parameters
    pub fn get_transform_matrix(&self, scale: f32, x_offset: f32, y_position: f32) -> [[f32; 3]; 3] {
        // Calculate the center of the image
        let center_x = self.width as f32 / 2.0;
        let center_y = self.height as f32 / 2.0;
        
        // Determine if we're in Telegram circular format
        let is_telegram = self.width == self.height;
        
        // Calculate the vertical position relative to the center
        let relative_y = (y_position - center_y) / center_y;
        
        // IMPROVED: More sophisticated skew calculation for circular format
        // For Telegram format, use a reduced skew angle and adjust based on position
        // Retain a noticeable skew even in Telegram circular mode so the 3-D
        // effect remains obvious.  We still attenuate near the extreme edges
        // to avoid unreadable glyphs, but much less aggressively than the
        // previous implementation.
        let skew_factor = if is_telegram {
            let distance_from_center = relative_y.abs();
            if distance_from_center < 0.3 {
                1.0   // Full skew in the centre band
            } else if distance_from_center < 0.6 {
                0.8   // Mid band
            } else {
                0.6   // Near the brim of the circle
            }
        } else {
            1.0
        };
        
        // Apply a more gradual skew reduction based on vertical position
        // This creates a more natural perspective effect
        let position_factor = 1.0 - relative_y.abs() * 0.5;
        
        // Calculate the final skew angle
        let skew_angle = self.skew * skew_factor * position_factor;
        
        // IMPROVED: More sophisticated depth calculation for circular format
        // Calculate the perspective depth factor with position-based adjustments
        let depth_factor = if is_telegram {
            // Adjust depth based on position in the circle
            let distance_from_center = relative_y.abs();
            if distance_from_center < 0.3 {
                // Central region - use 60% of standard depth
                self.depth * 0.6
            } else if distance_from_center < 0.6 {
                // Middle region - use 40% of standard depth
                self.depth * 0.4
            } else {
                // Edge region - use 30% of standard depth
                self.depth * 0.3
            }
        } else {
            // Standard format - use full depth
            self.depth
        };
        
        // Calculate the perspective scale based on the y position
        // IMPROVED: More gradual perspective scaling for better text distribution
        let perspective_scale = if is_telegram {
            // For circular format, use a more gradual perspective scale
            // This prevents text from becoming too small at the edges
            1.0 - (relative_y * depth_factor * 0.7).abs()
        } else {
            // Standard format - use normal perspective scaling
            1.0 - (relative_y * depth_factor).abs()
        };
        
        // Apply the scale factor to the perspective scale
        let final_scale = scale * perspective_scale;
        
        // IMPROVED: More sophisticated rotation for circular format
        // Calculate the rotation angle (subtle z-axis rotation for 3D effect)
        let rotation_angle = if is_telegram {
            // Apply a subtle rotation based on vertical position for circular format
            // More rotation at the edges, less in the center
            let rotation_factor = relative_y.abs() * 0.08;
            self.rotation + relative_y * rotation_factor
        } else {
            self.rotation
        };
        
        // Calculate the sin and cos of the angles
        let skew_sin = skew_angle.sin();
        let skew_cos = skew_angle.cos();
        let rot_sin = rotation_angle.sin();
        let rot_cos = rotation_angle.cos();
        
        // IMPROVED: Better x-position calculation for circular format
        // Calculate the final position with better centering for circular format
        // In practice the original horizontal-adjustment logic pushed all text
        // too far right, creating a wide left margin.  We now rely solely on
        // the layout engine’s centring so that `x_offset` is correct already.
        let x_pos = x_offset;
        
        // Create the transformation matrix
        // This combines scaling, rotation, skew, and translation
        [
            [
                final_scale * rot_cos,
                final_scale * rot_sin,
                0.0,
            ],
            [
                final_scale * -rot_sin + skew_sin * final_scale,
                final_scale * rot_cos + skew_cos * final_scale,
                0.0,
            ],
            [
                x_pos,
                y_position,
                1.0,
            ],
        ]
    }
    
    /// Apply a circular mask to an image (for Telegram stickers)
    pub fn apply_circular_mask(&self, image: &mut RgbaImage) -> Result<(), CodeSkewError> {
        let width = image.width();
        let height = image.height();
        
        // Ensure the image is square
        if width != height {
            return Err(CodeSkewError::RenderingError(
                "Cannot apply circular mask to non-square image".to_string(),
            ));
        }
        
        // Calculate the center and radius
        let center_x = width as f32 / 2.0;
        let center_y = height as f32 / 2.0;
        let radius = width as f32 / 2.0;
        
        // IMPROVED: Enhanced anti-aliasing for smoother circle edges
        // Apply the mask with improved edge smoothing
        for y in 0..height {
            for x in 0..width {
                // Calculate the distance from the center
                let dx = x as f32 - center_x;
                let dy = y as f32 - center_y;
                let distance = (dx * dx + dy * dy).sqrt();
                
                // If outside the circle, make transparent
                if distance > radius {
                    image.put_pixel(x, y, Rgba([0, 0, 0, 0]));
                }
                // If near the edge, apply a soft gradient for anti-aliasing
                // IMPROVED: Wider anti-aliasing region (3.0 pixels instead of 2.0)
                else if distance > radius - 3.0 {
                    // IMPROVED: Smoother alpha transition with cubic easing
                    let t = (radius - distance) / 3.0;
                    let alpha = t * t * (3.0 - 2.0 * t); // Cubic easing function
                    
                    let pixel = image.get_pixel(x, y);
                    let new_alpha = (pixel[3] as f32 * alpha).min(255.0) as u8;
                    image.put_pixel(x, y, Rgba([pixel[0], pixel[1], pixel[2], new_alpha]));
                }
            }
        }
        
        Ok(())
    }
    
    /// Apply a circular mask with debug visualization
    pub fn apply_circular_mask_debug(&self, image: &mut RgbaImage) -> Result<(), CodeSkewError> {
        let width = image.width();
        let height = image.height();
        
        // Ensure the image is square
        if width != height {
            return Err(CodeSkewError::RenderingError(
                "Cannot apply circular mask to non-square image".to_string(),
            ));
        }
        
        // Calculate the center and radius
        let center_x = width as f32 / 2.0;
        let center_y = height as f32 / 2.0;
        let radius = width as f32 / 2.0;
        
        // Apply the mask with debug visualization
        for y in 0..height {
            for x in 0..width {
                // Calculate the distance from the center
                let dx = x as f32 - center_x;
                let dy = y as f32 - center_y;
                let distance = (dx * dx + dy * dy).sqrt();
                
                // If outside the circle, make transparent
                if distance > radius {
                    image.put_pixel(x, y, Rgba([0, 0, 0, 0]));
                }
                // If near the edge, apply a soft gradient for anti-aliasing
                else if distance > radius - 3.0 {
                    let t = (radius - distance) / 3.0;
                    let alpha = t * t * (3.0 - 2.0 * t); // Cubic easing function
                    
                    let pixel = image.get_pixel(x, y);
                    let new_alpha = (pixel[3] as f32 * alpha).min(255.0) as u8;
                    
                    // Add a red border for debugging
                    if distance > radius - 1.0 {
                        image.put_pixel(x, y, Rgba([255, 0, 0, new_alpha]));
                    } else {
                        image.put_pixel(x, y, Rgba([pixel[0], pixel[1], pixel[2], new_alpha]));
                    }
                }
                // Add debug grid lines
                else if (dx.abs() < 1.0 || dy.abs() < 1.0) && (x % 32 == 0 || y % 32 == 0) {
                    image.put_pixel(x, y, Rgba([0, 255, 0, 128]));
                }
                // Add debug center marker
                else if dx.abs() < 5.0 && dy.abs() < 5.0 {
                    image.put_pixel(x, y, Rgba([255, 0, 255, 128]));
                }
                // Add debug concentric circles
                else if (distance - radius * 0.25).abs() < 1.0 || 
                         (distance - radius * 0.5).abs() < 1.0 || 
                         (distance - radius * 0.75).abs() < 1.0 {
                    image.put_pixel(x, y, Rgba([0, 128, 255, 128]));
                }
            }
        }
        
        Ok(())
    }
    
    /// Transform a point using the perspective transformation
    pub fn transform_point(&self, x: f32, y: f32) -> (f32, f32) {
        // Use a default scale of 1.0
        self.transform_point_with_scale(x, y, 1.0)
    }
    
    /// Transform a point using the perspective transformation with scale
    pub fn transform_point_with_scale(&self, x: f32, y: f32, scale: f32) -> (f32, f32) {
        // Get the transformation matrix
        let matrix = self.get_transform_matrix(scale, x, y);
        
        // Apply the transformation
        let tx = matrix[0][0] * x + matrix[0][1] * y + matrix[2][0];
        let ty = matrix[1][0] * x + matrix[1][1] * y + matrix[2][1];
        
        (tx, ty)
    }
}
