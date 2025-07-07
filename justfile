# Codeskew justfile - Simple commands for 3D code rendering

# Build the project
build:
    cargo build --release

# Install the binary globally
install:
    cargo install --path .

# AWESOME skew command - live animated WGPU preview with 3D skewed code!
skew file="src/main.rs":
    cargo run --release -- "{{file}}" --format wgpu --live --shader codeskew_unified

# Skew with custom angle and perspective
skew-custom file output="output.png" angle="15" perspective="0.3":
    cargo run --release -- "{{file}}" -o "{{output}}" --angle {{angle}} --perspective {{perspective}}

# Create a Telegram sticker (512x512 round PNG)
telegram file output="sticker.png":
    cargo run --release -- "{{file}}" -T -o "{{output}}"

# Create an animated GIF
animate file output="animation.gif" duration="3":
    cargo run --release -- "{{file}}" --animate --duration {{duration}} -o "{{output}}"

# Skew with custom gradient background
gradient file output="output.png" colors="blue,purple":
    cargo run --release -- "{{file}}" --gradient "{{colors}}" -o "{{output}}"

# Show help
help:
    cargo run --release -- --help

# Run tests
test:
    cargo test

# Clean build artifacts
clean:
    cargo clean

# Example usage with a sample file
example:
    echo 'fn main() {\n    println!("Hello, 3D world!");\n}' > sample.rs
    cargo run --release -- sample.rs -o example.png
    @echo "Created example.png from sample.rs"
    rm sample.rs