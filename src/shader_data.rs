//! Shader data generation for unified compute shaders
//! Converts syntax-highlighted code into GPU-friendly buffers

use crate::layout::PositionedLine;

pub const TERMINAL_ROWS: usize = 30;
pub const TERMINAL_COLS: usize = 80;

/// ASCII grid for terminal characters
pub type TerminalGrid = [[u32; TERMINAL_COLS]; TERMINAL_ROWS];

/// Color grid for syntax highlighting (packed RGB)
pub type ColorGrid = [[u32; TERMINAL_COLS]; TERMINAL_ROWS];

/// Shader data containing ASCII characters and colors for GPU rendering
#[derive(Debug, Clone)]
pub struct ShaderTextData {
    pub terminal_grid: TerminalGrid,
    pub color_grid: ColorGrid,
    pub rows_used: usize,
    pub cols_used: usize,
}

impl ShaderTextData {
    /// Create new empty shader text data
    pub fn new() -> Self {
        Self {
            terminal_grid: [[0; TERMINAL_COLS]; TERMINAL_ROWS],
            color_grid: [[0xFFFFFF; TERMINAL_COLS]; TERMINAL_ROWS], // Default white
            rows_used: 0,
            cols_used: 0,
        }
    }

    /// Generate shader data from positioned lines (syntax-highlighted layout)
    pub fn from_layout(layout: &[PositionedLine]) -> Self {
        let mut data = Self::new();
        let mut row = 0;

        for line in layout.iter().take(TERMINAL_ROWS) {
            let mut col = 0;
            
            for styled_char in &line.chars {
                if col >= TERMINAL_COLS {
                    break; // Line too long for terminal
                }

                // Store ASCII character
                data.terminal_grid[row][col] = styled_char.char as u32;

                // Pack RGB color into u32 (0xRRGGBB format)
                let r = (styled_char.color.r as u32) << 16;
                let g = (styled_char.color.g as u32) << 8;
                let b = styled_char.color.b as u32;
                data.color_grid[row][col] = r | g | b;

                col += 1;
            }

            data.cols_used = data.cols_used.max(col);
            row += 1;
        }

        data.rows_used = row;
        
        println!("🔤 Generated shader text data: {}x{} characters", data.rows_used, data.cols_used);
        
        data
    }

    /// Convert to flat buffer for GPU upload
    pub fn to_terminal_buffer(&self) -> Vec<u32> {
        let mut buffer = Vec::with_capacity(TERMINAL_ROWS * TERMINAL_COLS);
        
        for row in &self.terminal_grid {
            buffer.extend_from_slice(row);
        }
        
        buffer
    }

    /// Convert to flat color buffer for GPU upload
    pub fn to_color_buffer(&self) -> Vec<u32> {
        let mut buffer = Vec::with_capacity(TERMINAL_ROWS * TERMINAL_COLS);
        
        for row in &self.color_grid {
            buffer.extend_from_slice(row);
        }
        
        buffer
    }

    /// Get buffer sizes for GPU allocation
    pub fn buffer_size() -> usize {
        TERMINAL_ROWS * TERMINAL_COLS * std::mem::size_of::<u32>()
    }
}

impl Default for ShaderTextData {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{PositionedLine, StyledChar};
    use crate::gradient::ColorRgb;

    #[test]
    fn test_shader_data_generation() {
        let mut line = PositionedLine {
            chars: vec![
                StyledChar {
                    char: 'f',
                    color: ColorRgb { r: 255, g: 100, b: 50 }, // Orange
                },
                StyledChar {
                    char: 'n',
                    color: ColorRgb { r: 100, g: 255, b: 100 }, // Green
                },
            ],
            x: 0.0,
            y: 0.0,
        };

        let layout = vec![line];
        let data = ShaderTextData::from_layout(&layout);

        assert_eq!(data.terminal_grid[0][0], 'f' as u32);
        assert_eq!(data.terminal_grid[0][1], 'n' as u32);
        assert_eq!(data.color_grid[0][0], 0xFF6432); // Orange packed
        assert_eq!(data.color_grid[0][1], 0x64FF64); // Green packed
        assert_eq!(data.rows_used, 1);
        assert_eq!(data.cols_used, 2);
    }
}