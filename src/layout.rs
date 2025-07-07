use crate::config::Config;
use crate::error::CodeSkewError;
use crate::highlight::StyledLine;
use anyhow::Result;

/// A styled character with color information
#[derive(Debug, Clone)]
pub struct StyledChar {
    pub char: char,
    pub color: CharColor,
}

/// Color information for a character
#[derive(Debug, Clone)]
pub struct CharColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// A line of text with position information
#[derive(Debug, Clone)]
pub struct PositionedLine {
    pub chars: Vec<StyledChar>,
    pub x: f32,
    pub y: f32,
    pub scale: f32, // Scale factor for this line
}

/// Layout engine for positioning code lines
pub struct LayoutEngine {
    config: Config,
}

impl LayoutEngine {
    /// Create a new layout engine
    pub fn new(config: &Config) -> Self {
        Self {
            config: config.clone(),
        }
    }
    
    /// Calculate the base font size for given content
    pub fn calculate_base_font_size(&self, highlighted_code: &[StyledLine]) -> f32 {
        // Count non-empty lines so we can size according to height.
        let non_empty_lines: Vec<&StyledLine> = highlighted_code
            .iter()
            .filter(|line| {
                !line.spans.is_empty() && line.spans.iter().any(|span| !span.text.trim().is_empty())
            })
            .collect();

        let line_count = non_empty_lines.len().max(1);

        // Find the longest line (in raw character count) so we can size according to width.
        let longest_line_chars = highlighted_code
            .iter()
            .map(|line| line.spans.iter().map(|s| s.text.len()).sum::<usize>())
            .max()
            .unwrap_or(0) as f32;

        let char_width_ratio = 0.7;

        // Font size constrained by available *height* (leave ~20 % top/bottom)
        let font_by_height = (self.config.height as f32 * 0.8) / (line_count as f32 * 1.4);

        // Font size constrained by available *width* (leave ~10 % margin)
        let font_by_width = if longest_line_chars > 0.0 {
            (self.config.width as f32 * 0.9) / (longest_line_chars * char_width_ratio)
        } else {
            font_by_height
        };

        // Use the tighter constraint so we satisfy both axes.
        let mut base_font_size = font_by_height.min(font_by_width);

        // Keep font size within sane limits
        base_font_size = base_font_size
            .max(8.0) // readability
            .min(self.config.fontsize * 10.0);
            
        base_font_size
    }

    /// Layout the highlighted code
    pub fn layout(
        &self,
        highlighted_code: &[StyledLine],
    ) -> Result<Vec<PositionedLine>, CodeSkewError> {
        // Check if there's any code to layout
        if highlighted_code.is_empty() {
            return Err(CodeSkewError::LayoutError("No code to layout".to_string()));
        }

        if self.config.telegram {
            self.layout_circular(highlighted_code)
        } else {
            self.layout_standard(highlighted_code)
        }
    }

    /// Layout code for circular Telegram format
    fn layout_circular(
        &self,
        highlighted_code: &[StyledLine],
    ) -> Result<Vec<PositionedLine>, CodeSkewError> {
        // Circle parameters
        let radius = self.config.width as f32 / 2.0;
        let center_x = radius;
        let center_y = radius;

        // Calculate non-empty lines for sizing
        let non_empty_lines: Vec<&StyledLine> = highlighted_code
            .iter()
            .filter(|line| {
                !line.spans.is_empty() && line.spans.iter().any(|span| !span.text.trim().is_empty())
            })
            .collect();

        let line_count = non_empty_lines.len().max(1);
        println!(
            "Circular layout: {} lines in {}px circle",
            line_count, self.config.width
        );

        // Improved font sizing for circular format
        // Use inscribed square approach for better space utilization
        let inscribed_square_side = radius * 2.0 / std::f32::consts::SQRT_2;
        let usable_height = inscribed_square_side * 0.85; // Use 85% for padding
        let optimal_font_size = usable_height / (line_count as f32 * 1.3);

        // Constrain font size to reasonable bounds
        let base_font_size = optimal_font_size
            .min(self.config.fontsize * 3.0) // Allow larger fonts for few lines
            .clamp(10.0, 24.0); // Clamp between minimum readable and maximum size

        println!("Circular font size: {base_font_size}");

        let line_height = base_font_size * 1.25; // Slightly tighter line spacing
        let char_width = base_font_size * 0.55; // Adjust for typical monospace ratio

        // Calculate total text block height
        let total_height = line_height * highlighted_code.len() as f32;
        let start_y = center_y - (total_height / 2.0) + line_height / 2.0;

        let mut positioned_lines = Vec::new();

        for (i, line) in highlighted_code.iter().enumerate() {
            let y_position = start_y + i as f32 * line_height;

            // Improved circular constraint calculation
            // Add padding from circle edge for better visibility
            let edge_padding = radius * 0.15; // 15% padding from edge
            let safe_radius = radius - edge_padding;

            // Calculate the maximum width available at this y position
            let y_offset = (y_position - center_y).abs();
            let max_half_width = if y_offset < safe_radius {
                (safe_radius * safe_radius - y_offset * y_offset).sqrt()
            } else {
                0.0
            };

            // Skip lines that would be completely outside the circle
            if max_half_width <= 0.0 {
                continue;
            }

            // Calculate line width
            let line_chars = line.spans.iter().map(|span| span.text.len()).sum::<usize>() as f32;
            let line_width = line_chars * char_width;

            // Apply intelligent scaling
            let available_width = max_half_width * 2.0 * 0.9; // Use 90% of available width
            let scale_factor = if line_width > available_width && available_width > 0.0 {
                // Scale down long lines but not too much
                (available_width / line_width).max(0.7)
            } else {
                1.0
            };

            // Center the line horizontally
            let actual_width = line_width * scale_factor;
            let x_offset = center_x - actual_width / 2.0;

            // Convert spans to styled characters
            let mut styled_chars = Vec::new();
            for span in &line.spans {
                for ch in span.text.chars() {
                    styled_chars.push(StyledChar {
                        char: ch,
                        color: CharColor {
                            r: span.style.foreground.0,
                            g: span.style.foreground.1,
                            b: span.style.foreground.2,
                        },
                    });
                }
            }

            // Add the positioned line with scaling information
            positioned_lines.push(PositionedLine {
                chars: styled_chars,
                x: x_offset,
                y: y_position,
                scale: scale_factor,
            });
        }

        Ok(positioned_lines)
    }

    /// Standard layout for rectangular formats
    fn layout_standard(
        &self,
        highlighted_code: &[StyledLine],
    ) -> Result<Vec<PositionedLine>, CodeSkewError> {
        // ------------------------------------------------------------------
        // Dynamically determine an appropriate font size so that *all* lines
        // fit inside the configured image dimensions.  The previous logic
        // only considered the vertical axis which meant that moderately long
        // lines were rendered far wider than the canvas, resulting in the
        // right-hand side being clipped ("missing braces" in the generated
        // image).  We now take the horizontal constraint into account as
        // well and pick the smaller of the two resulting font sizes.
        // ------------------------------------------------------------------

        // Calculate base font size using helper function
        let base_font_size = self.calculate_base_font_size(highlighted_code);
        
        // Count lines for layout calculations
        let line_count = highlighted_code
            .iter()
            .filter(|line| {
                !line.spans.is_empty() && line.spans.iter().any(|span| !span.text.trim().is_empty())
            })
            .count()
            .max(1);
        
        println!("Standard layout: {line_count} lines");
        println!("Chosen font size: {base_font_size:.2}");

        // Metric helpers
        // The text renderer receives a per-line `scale` further below, so we
        // don’t need a global one here – remove the earlier placeholder.
        let line_height = base_font_size * 1.4; // 140 % spacing
        let blank_line_height = line_height * 0.4; // compress blank lines
        let avg_char_width = base_font_size * 0.6; // monospace advance

        // --------------------------------------------------------------
        // Pass 1: how tall is the whole block once we compress blanks?
        // --------------------------------------------------------------
        let mut total_height = 0.0;
        for line in highlighted_code {
            let is_blank = line.spans.iter().all(|s| s.text.trim().is_empty());
            total_height += if is_blank {
                blank_line_height
            } else {
                line_height
            };
        }

        let mut current_y = (self.config.height as f32 - total_height) / 2.0;

        let mut positioned_lines = Vec::new();

        // --------------------------------------------------------------
        // Pass 2: assign positions + per-line scaling.
        // --------------------------------------------------------------
        for line in highlighted_code {
            let is_blank = line.spans.iter().all(|s| s.text.trim().is_empty());
            let effective_height = if is_blank {
                blank_line_height
            } else {
                line_height
            };

            // Width calculation (rough but good enough for monospace)
            let line_chars: usize = line.spans.iter().map(|s| s.text.len()).sum();
            let raw_width = line_chars as f32 * avg_char_width;

            let max_allowed = self.config.width as f32 * 0.9; // 5 % margin both sides
            let scale_factor = if raw_width > max_allowed {
                (max_allowed / raw_width).max(0.5)
            } else {
                1.0
            };

            let actual_width = raw_width * scale_factor;
            let x_offset = (self.config.width as f32 - actual_width) / 2.0;

            // Build character list (skip if blank)
            if !is_blank {
                let mut styled_chars = Vec::with_capacity(line_chars);
                for span in &line.spans {
                    for ch in span.text.chars() {
                        styled_chars.push(StyledChar {
                            char: ch,
                            color: CharColor {
                                r: span.style.foreground.0,
                                g: span.style.foreground.1,
                                b: span.style.foreground.2,
                            },
                        });
                    }
                }

                positioned_lines.push(PositionedLine {
                    chars: styled_chars,
                    x: x_offset,
                    y: current_y + effective_height / 2.0, // baseline roughly centre
                    scale: scale_factor,
                });
            }

            current_y += effective_height;
        }

        Ok(positioned_lines)
    }
}
