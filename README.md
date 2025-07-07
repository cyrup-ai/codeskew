# Codeskew

A production-quality tool for rendering code with 3D perspective effects.

## Features

- **3D Perspective Rendering**: Apply skew and perspective effects to code snippets
- **Multiple Output Formats**: PNG, SVG, GIF, and WebP (including Telegram-compatible stickers)
- **Syntax Highlighting**: Automatic language detection and syntax highlighting
- **Customizable Appearance**: Control colors, fonts, angles, and more
- **Animation Support**: Create animated code snippets
- **Gradient Backgrounds**: Apply beautiful gradient backgrounds to your code
- **Telegram Sticker Export**: Create 512x512 round PNG images for Telegram compatibility

## Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/codeskew.git
cd codeskew

# Build the project
cargo build --release

# The binary will be available at target/release/codeskew
```

## Usage

```bash
# Basic usage
codeskew path/to/code.rs -o output.png

# Telegram sticker export
codeskew path/to/code.rs -T -o telegram_sticker.png

# Custom angle and perspective
codeskew path/to/code.rs --angle 30 --perspective 0.5 -o output.png

# Animation
codeskew path/to/code.rs --animate -o output.gif

# SVG output
codeskew path/to/code.rs -o output.svg

# Custom theme
codeskew path/to/code.rs --theme monokai -o output.png

# Custom font and size
codeskew path/to/code.rs --font "Fira Code" --fontsize 14 -o output.png

# Custom gradient background
codeskew path/to/code.rs --gradient "blue,purple" -o output.png

# Help
codeskew --help
```

## Command Line Options

```
Usage: codeskew [OPTIONS] <INPUT_FILE>

Arguments:
  <INPUT_FILE>  Path to the source code file to render

Options:
  -o, --output <o>              Output file path [default: output.png]
  -f, --format <FORMAT>              Output format (png, svg, gif, webp) [default: auto]
  -T, --telegram                     Generate Telegram-compatible sticker (512x512 round PNG)
  -a, --angle <ANGLE>                Skew angle in degrees [default: 15]
  -p, --perspective <PERSPECTIVE>    Perspective depth factor [default: 0.3]
  -w, --width <WIDTH>                Output image width [default: 800]
  -h, --height <HEIGHT>              Output image height [default: 600]
      --font <FONT>                  Font name [default: Menlo]
      --fontsize <FONTSIZE>          Font size [default: 14]
      --theme <THEME>                Syntax highlighting theme [default: base16-ocean.dark]
      --gradient <GRADIENT>          Gradient colors (comma-separated) [default: #2b303b,#16181d]
      --animate                      Create an animation
      --duration <DURATION>          Animation duration in seconds [default: 3]
      --fps <FPS>                    Animation frames per second [default: 30]
      --blur <BLUR>                  Apply blur effect [default: 0]
  -h, --help                         Print help
  -V, --version                      Print version
```

## Telegram Sticker Mode

The `-T` or `--telegram` flag enables Telegram sticker mode, which:

1. Sets the output dimensions to 512x512 pixels
2. Creates a circular mask for the sticker
3. Generates a static PNG image with proper transparency
4. Optimizes the layout and scaling for circular format
5. Enhances contrast for better visibility in the circular mask

Note: While the original intention was to create animated WebP files for Telegram, the current implementation uses static PNG files due to encoder limitations. The PNG output is fully compatible with Telegram stickers and provides excellent quality.

Example:
```bash
codeskew cool_code.py -T -o my_code_sticker.png
```

## Examples

### Basic PNG Output
```bash
codeskew hello.rs -o hello.png
```

### Telegram Sticker
```bash
codeskew algorithm.py -T -o algo_sticker.png
```

### Custom Styling
```bash
codeskew main.js --angle 25 --perspective 0.4 --gradient "red,orange" --font "Fira Code" -o custom.png
```

### Animation
```bash
codeskew cool_algorithm.cpp --animate --duration 5 --fps 60 -o animation.gif
```

## Font Fallback

If the specified font is not available on your system, codeskew will automatically fall back to:
1. DejaVu Sans Mono
2. Liberation Mono
3. Courier New
4. Consolas
5. Any available monospace font

## Recent Updates

- Fixed text rendering to use actual glyphs with proper kerning and anti-aliasing
- Improved contrast and visibility of code in all output formats
- Enhanced layout engine for better positioning in circular formats
- Added static PNG fallback for Telegram stickers with proper circular masking
- Optimized perspective transformation for better readability in circular formats

## License

MIT

## Acknowledgements

Codeskew incorporates code from the [wgpu-compute-toy](https://github.com/compute-toys/wgpu-compute-toy) project, which is the compute shader engine for compute.toys. See the full [ACKNOWLEDGEMENTS.md](./ACKNOWLEDGEMENTS.md) for more details on the usage and licensing.
