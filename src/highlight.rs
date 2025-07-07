use crate::error::CodeSkewError;
use anyhow::Result;
use std::path::Path;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxReference;
use syntect::parsing::SyntaxSet;

/// A styled span of text with syntax highlighting
#[derive(Debug, Clone)]
pub struct StyledSpan {
    pub text: String,
    #[allow(dead_code)] // Public API - span positioning for external use
    pub start: usize,
    #[allow(dead_code)] // Public API - span positioning for external use
    pub end: usize,
    pub style: SpanStyle,
}

/// Style information for a span of text
#[derive(Debug, Clone)]
pub struct SpanStyle {
    pub foreground: (u8, u8, u8),
    #[allow(dead_code)] // Public API - background color for external use
    pub background: (u8, u8, u8),
    #[allow(dead_code)] // Public API - text styling for external use
    pub is_bold: bool,
    #[allow(dead_code)] // Public API - text styling for external use
    pub is_italic: bool,
    #[allow(dead_code)] // Public API - text styling for external use
    pub is_underline: bool,
}

/// A line of styled spans
#[derive(Debug, Clone)]
pub struct StyledLine {
    pub spans: Vec<StyledSpan>,
    #[allow(dead_code)] // Public API - line numbering for external use
    pub line_number: usize,
}

/// Syntax highlighter for code
pub struct SyntaxHighlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    default_theme: String,
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

impl SyntaxHighlighter {
    /// Create a new syntax highlighter
    pub fn new() -> Self {
        // Load the syntax definitions and themes
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();

        // Use a theme that's guaranteed to be available
        let default_theme = "base16-ocean.dark".to_string();

        Self {
            syntax_set,
            theme_set,
            default_theme,
        }
    }

    /// Get the syntax for a file path
    fn get_syntax_for_path(&self, path: &Path) -> Option<&SyntaxReference> {
        let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

        self.get_syntax_for_extension(extension)
    }

    /// Get the syntax for a file extension
    fn get_syntax_for_extension(&self, extension: &str) -> Option<&SyntaxReference> {
        self.syntax_set.find_syntax_by_extension(extension)
    }

    /// Convert a syntect Style to our SpanStyle
    fn convert_style(&self, style: &Style) -> SpanStyle {
        SpanStyle {
            foreground: (style.foreground.r, style.foreground.g, style.foreground.b),
            background: (style.background.r, style.background.g, style.background.b),
            is_bold: style
                .font_style
                .contains(syntect::highlighting::FontStyle::BOLD),
            is_italic: style
                .font_style
                .contains(syntect::highlighting::FontStyle::ITALIC),
            is_underline: style
                .font_style
                .contains(syntect::highlighting::FontStyle::UNDERLINE),
        }
    }

    /// Get available theme names
    pub fn get_available_themes(&self) -> Vec<String> {
        self.theme_set.themes.keys().cloned().collect()
    }

    /// Set the default theme
    #[allow(dead_code)] // Public API - theme configuration for external use
    pub fn set_default_theme(&mut self, theme_name: String) {
        self.default_theme = theme_name;
    }

    /// Find a theme by name (case-insensitive)
    /// Returns a key from the theme_set that matches the requested theme name
    fn find_theme(&self, theme_name: &str) -> Option<String> {
        // First try exact match
        if self.theme_set.themes.contains_key(theme_name) {
            return Some(theme_name.to_string());
        }

        // Try case-insensitive match
        for key in self.theme_set.themes.keys() {
            if key.to_lowercase() == theme_name.to_lowercase() {
                return Some(key.to_string());
            }
        }

        None
    }

    /// Highlight code with syntax highlighting
    pub fn highlight(
        &self,
        source: &str,
        path: &Path,
        theme_name: &str,
    ) -> Result<Vec<StyledLine>> {
        // Get the syntax for the file
        let syntax = self.get_syntax_for_path(path).ok_or_else(|| {
            CodeSkewError::SyntaxError(format!("No syntax found for file: {}", path.display()))
        })?;

        // Find the theme by name (case-insensitive)
        let theme_key = self
            .find_theme(theme_name)
            .or_else(|| self.find_theme(&self.default_theme))
            .ok_or_else(|| {
                CodeSkewError::SyntaxError(format!(
                    "Theme not found: {}. Available themes: {}",
                    theme_name,
                    self.get_available_themes().join(", ")
                ))
            })?;

        // Get the theme
        let theme = &self.theme_set.themes[&theme_key];

        // Create the highlighter
        let mut highlighter = syntect::easy::HighlightLines::new(syntax, theme);

        // Highlight each line
        let mut styled_lines = Vec::new();

        for (line_idx, line) in source.lines().enumerate() {
            let highlights = highlighter.highlight_line(line, &self.syntax_set)?;

            let mut spans = Vec::new();
            let mut start = 0;

            for (style, text) in highlights {
                let end = start + text.len();

                spans.push(StyledSpan {
                    text: text.to_string(),
                    start,
                    end,
                    style: self.convert_style(&style),
                });

                start = end;
            }

            styled_lines.push(StyledLine {
                spans,
                line_number: line_idx + 1,
            });
        }

        Ok(styled_lines)
    }
}
