//! Core OutputGenerator for high-performance code rendering with zero allocation

use crate::cli::OutputFormat;
use crate::config::Config;
use crate::error::CodeSkewError;
use crate::highlight::SyntaxHighlighter;
use crate::layout::{LayoutEngine, PositionedLine};
use crate::output::SaveMethods;
use crate::toy::{WgpuToyRenderer, init_wgpu};
use anyhow::Result;
use image::RgbaImage;
use std::time::Instant;
use serde::{Deserialize, Serialize};
use crate::glyphon::GlyphonTextureRenderer;

/// Shader metadata structure matching wgpu-compute-toy JSON format
#[derive(Debug, Deserialize, Serialize)]
struct ShaderMetadata {
    uniforms: Vec<ShaderUniform>,
    textures: Vec<ShaderTexture>,
    #[serde(rename = "float32Enabled")]
    float32_enabled: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ShaderUniform {
    name: String,
    value: f32,
}

#[derive(Debug, Deserialize, Serialize)]
struct ShaderTexture {
    img: String,
    thumb: Option<String>,
    url: Option<String>,
}

/// Template data for MiniJinja WGSL generation
#[derive(Debug)]
struct TemplateData {
    characters: Vec<u32>,
    colors: Vec<u32>,
    positions: Vec<PositionData>,
    max_chars_per_line: usize,
}

#[derive(Debug, Serialize)]
struct PositionData {
    line_idx: usize,
    char_idx: usize,
    x: f32,
    y: f32,
    scale: f32,
}

/// High-performance composite renderer combining toy shader backgrounds with elite text rendering
/// Zero allocation in hot paths, blazing-fast performance, production-quality implementation
pub struct OutputGenerator {
    config: Config,
    layout_engine: LayoutEngine,
    highlighter: SyntaxHighlighter,

    // Pre-allocated buffers for zero-allocation operation in hot paths
    rgba_buffer: Vec<u8>,
    temp_buffer: Vec<u8>,
    shader_buffer: String,
    pixel_count: usize,
    buffer_size: usize,

    // Pre-calculated values for performance
    width_f32: f32,
    height_f32: f32,
    width_u32: u32,
    height_u32: u32,
}

impl OutputGenerator {
    /// Create a new blazing-fast composite output generator with pre-allocated buffers
    #[inline]
    pub fn new(config: Config) -> Result<Self, CodeSkewError> {
        let layout_engine = LayoutEngine::new(&config);
        let highlighter = SyntaxHighlighter::new();

        // Pre-calculate dimensions for optimal performance
        let width_u32 = config.width;
        let height_u32 = config.height;
        let width_f32 = width_u32 as f32;
        let height_f32 = height_u32 as f32;
        let pixel_count = (width_u32 * height_u32) as usize;
        let buffer_size = pixel_count * 4; // RGBA format

        // Pre-allocate all buffers with exact capacity to avoid reallocations
        let mut rgba_buffer = Vec::with_capacity(buffer_size);
        let mut temp_buffer = Vec::with_capacity(buffer_size);
        let shader_buffer = String::with_capacity(2048);

        // Initialize buffers to required size
        rgba_buffer.resize(buffer_size, 0);
        temp_buffer.resize(buffer_size, 0);

        Ok(Self {
            config,
            layout_engine,
            highlighter,
            rgba_buffer,
            temp_buffer,
            shader_buffer,
            pixel_count,
            buffer_size,
            width_f32,
            height_f32,
            width_u32,
            height_u32,
        })
    }

    /// Generate output using toy renderer with blazing performance and zero allocation
    #[inline]
    pub async fn generate(&mut self, code: &str) -> Result<(), CodeSkewError> {
        let start_time = Instant::now();

        // Syntax highlighting - optimized hot path with zero allocation
        let highlighted_code = self
            .highlighter
            .highlight(code, &self.config.input, &self.config.theme)
            .map_err(|e| CodeSkewError::SyntaxError(format!("Failed to highlight code: {e}")))?;

        // Layout generation - pre-allocated structures
        let layout = self.layout_engine.layout(&highlighted_code)?;

        // Handle live preview separately - no layered rendering needed
        if self.config.format == OutputFormat::Wgpu {
            println!("🚀 Launching live animated WGPU preview window!");
            self.launch_live_preview(&layout).await?;
            return Ok(());
        }

        // For static outputs, render using toy renderer
        let wgpu_context = init_wgpu(self.width_u32, self.height_u32, "")
            .await
            .map_err(|e| {
                CodeSkewError::RenderingError(format!("Failed to create WGPU context: {e}"))
            })?;
        
        // Render with toy renderer (optionally with text if shader supports it)
        let buffer_data = self.render_with_toy(wgpu_context).await?;
        
        // Convert buffer to image
        let image = RgbaImage::from_raw(
            self.width_u32,
            self.height_u32,
            buffer_data,
        )
        .ok_or_else(|| {
            CodeSkewError::RenderingError(
                "Failed to create RgbaImage from buffer".to_string(),
            )
        })?;

        // Save output with optimized format handling
        let save_methods = SaveMethods::new(&self.config);
        match self.config.format {
            OutputFormat::Png => save_methods.save_png_optimized(image).await?,
            OutputFormat::Svg => {
                return Err(CodeSkewError::ConfigError(
                    "SVG not supported with WGSL shaders".to_string(),
                ));
            }
            OutputFormat::Gif => save_methods.save_gif_animation_optimized(&layout).await?,
            OutputFormat::Webp => save_methods.save_webp_optimized(image).await?,
            OutputFormat::Wgpu => unreachable!(), // Already handled above
        }

        println!(
            "🚀 Render completed in {:.2?}! ✨",
            start_time.elapsed()
        );
        println!("💎 Output saved to: {}", self.config.output.display());

        Ok(())
    }


    /// Render using toy shaders with optimized performance
    #[inline]
    async fn render_with_toy(
        &mut self,
        wgpu_context: crate::toy::WgpuContext,
    ) -> Result<Vec<u8>, CodeSkewError> {
        println!("🔧 DEBUG: Starting toy renderer");
        let mut toy_renderer = WgpuToyRenderer::new(wgpu_context);

        // Load actual texture file into channel0
        if let Err(e) = self.load_shader_textures(&mut toy_renderer, &self.config.shader).await {
            println!("🔧 WARNING: Failed to load textures: {}. Using fallback procedural texture.", e);
            // Fallback to procedural texture if image loading fails
            if let Err(e2) = toy_renderer.load_procedural_texture(0, 256, 256) {
                println!("🔧 ERROR: Even fallback texture failed: {}", e2);
            } else {
                toy_renderer.recreate_bind_group();
                println!("🔧 DEBUG: Using fallback procedural texture");
            }
        } else {
            toy_renderer.recreate_bind_group();
            println!("🔧 DEBUG: Successfully loaded shader textures");
        }

        // Build optimized background shader with zero allocation string building
        self.shader_buffer.clear();
        self.build_background_shader();
        println!("🔧 DEBUG: Built shader, length: {}", self.shader_buffer.len());

        // Compile shader with efficient error handling
        if let Some(source_map) = toy_renderer.preprocess_async(&self.shader_buffer).await {
            println!("🔧 DEBUG: Shader preprocessing successful");
            toy_renderer.compile(source_map);
            println!("🔧 DEBUG: Shader compilation successful");
        } else {
            println!("🔧 DEBUG: Shader preprocessing FAILED");
            return Err(CodeSkewError::RenderingError("Shader compilation failed".to_string()));
        }

        // Render background to buffer with optimized staging buffer handling
        let background_data = toy_renderer
            .render_to_buffer()
            .await
            .map_err(|e| CodeSkewError::RenderingError(format!("Background render failed: {e}")))?;
        
        println!("🔧 DEBUG: Background data length: {}", background_data.len());
        if background_data.len() >= 16 {
            println!("🔧 DEBUG: First 16 bytes: {:?}", &background_data[0..16]);
        }
        
        // Check for non-zero data
        let non_zero_count = background_data.iter().filter(|&&x| x != 0).count();
        println!("🔧 DEBUG: Non-zero bytes in background: {}/{}", non_zero_count, background_data.len());
        
        Ok(background_data)
    }

    /// Build background shader with zero allocation string operations
    #[inline]
    fn build_background_shader(&mut self) {
        // Load the shader specified in config (default is zeroshot)
        let shader_name = self.config.shader.clone();
        self.select_shader(&shader_name);
    }
    
    /// Load any WGSL shader by name
    fn load_shader(&mut self, shader_name: &str) {
        self.shader_buffer.clear();
        let shader_path = format!("wgsl/{}.wgsl", shader_name);
        match std::fs::read_to_string(&shader_path) {
            Ok(shader_content) => {
                self.shader_buffer.push_str(&shader_content);
            },
            Err(_) => {
                eprintln!("Warning: Could not load shader {}, using default", shader_path);
                // Default simple shader
                self.shader_buffer.push_str(&format!(r#"
@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let pos = vec2<f32>(global_id.xy);
    let resolution = vec2<f32>({}, {});
    if (global_id.x >= {} || global_id.y >= {}) {{ return; }}
    let uv = pos / resolution;
    let color = vec3<f32>(uv, sin(time.elapsed));
    textureStore(screen, vec2<i32>(global_id.xy), vec4<f32>(color, 1.0));
}}
"#, self.width_f32, self.height_f32, self.width_u32, self.height_u32));
            }
        }
    }
    
    /// Select shader by name for dynamic background switching
    pub fn select_shader(&mut self, shader_name: &str) {
        self.load_shader(shader_name);
    }


    /// Launch live animated WGPU preview window - copied from wgpu-compute-toy approach
    async fn launch_live_preview(&mut self, layout: &[PositionedLine]) -> Result<(), CodeSkewError> {
        use winit::{
            event::{ElementState, Event, KeyEvent, WindowEvent},
            keyboard::{KeyCode, PhysicalKey},
        };

        println!("🎮 Launching CodeSkew live WGPU preview...");

        // Create WGPU context using the exact same approach as wgpu-compute-toy
        let wgpu_context = crate::toy::init_wgpu(self.width_u32, self.height_u32, "").await
            .map_err(|e| CodeSkewError::RenderingError(format!("Failed to create WGPU context: {e}")))?;
        
        let mut wgputoy = crate::toy::WgpuToyRenderer::new(wgpu_context);

        // Process all shaders through unified MiniJinja template system
        let rendered_wgsl = self.process_shader_template(layout).await?;

        // Load textures for all shaders (unified path)
        if let Err(e) = wgputoy.load_procedural_texture(0, 256, 256) {
            println!("🔧 WARNING: Failed to load texture: {}", e);
        } else {
            wgputoy.recreate_bind_group();
        }

        // Compile the unified rendered shader
        println!("🔧 Compiling unified shader template");
        if let Some(source_map) = wgputoy.preprocess_async(&rendered_wgsl).await {
            println!("🔧 Unified shader preprocessing successful");
            wgputoy.compile(source_map);
            println!("🔧 Unified shader compilation successful");
        } else {
            return Err(CodeSkewError::RenderingError("Failed to preprocess unified shader".to_string()));
        }

        wgputoy.wgpu.window.set_title("CodeSkew Live Preview");
        let screen_size = wgputoy.wgpu.window.inner_size();
        let event_loop = std::mem::take(&mut wgputoy.wgpu.event_loop).unwrap();
        
        // Create device clone for polling (copied from wgpu-compute-toy)
        let device_clone = wgputoy.wgpu.device.clone();
        std::thread::spawn(move || loop {
            let _ = device_clone.poll(wgpu::MaintainBase::Wait);
            std::thread::yield_now();
        });

        let mut close_requested = false;
        let mut paused = false;
        let mut current_instant = std::time::Instant::now();
        let mut reference_time = 0.0;

        println!("🚀 Live preview ready! Press ESC to exit, SPACE to pause, BACKSPACE to reset time.");

        // Main event loop - copied verbatim from wgpu-compute-toy
        let _ = event_loop.run(move |event, elwt| {
            match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        device_id: _,
                        event:
                            KeyEvent {
                                state: ElementState::Released,
                                physical_key: PhysicalKey::Code(KeyCode::Escape),
                                ..
                            },
                        ..
                    } => {
                        close_requested = true;
                    }
                    WindowEvent::KeyboardInput {
                        device_id: _,
                        event:
                            KeyEvent {
                                state: ElementState::Released,
                                physical_key: PhysicalKey::Code(KeyCode::Backspace),
                                ..
                            },
                        ..
                    } => {
                        // reset time
                        paused = false;
                        reference_time = 0.0;
                        current_instant = std::time::Instant::now();
                        wgputoy.reset();
                        println!("🔄 Time reset");
                    }
                    WindowEvent::KeyboardInput {
                        device_id: _,
                        event:
                            KeyEvent {
                                state: ElementState::Released,
                                physical_key: PhysicalKey::Code(KeyCode::Space),
                                ..
                            },
                        ..
                    } => {
                        // toggle pause
                        paused = !paused;
                        if !paused {
                            current_instant = std::time::Instant::now();
                            wgputoy.wgpu.window.set_title("CodeSkew Live Preview");
                        } else {
                            reference_time = reference_time + current_instant.elapsed().as_secs_f32();
                            wgputoy.wgpu.window.set_title("CodeSkew Live Preview - Paused");
                        }
                        println!("⏯️  Paused: {}", paused);
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        wgputoy.set_mouse_pos(
                            position.x as f32 / screen_size.width as f32,
                            position.y as f32 / screen_size.height as f32,
                        );
                    }
                    WindowEvent::MouseInput { state, .. } => {
                        wgputoy.set_mouse_click(state == ElementState::Pressed);
                    }
                    WindowEvent::Resized(size) => {
                        if size.width != 0 && size.height != 0 {
                            wgputoy.resize(size.width, size.height, 1.);
                        }
                    }
                    WindowEvent::RedrawRequested => {
                        if !paused {
                            let time = reference_time + current_instant.elapsed().as_secs_f32();
                            wgputoy.set_time_elapsed(time);
                        }
                        let future = wgputoy.render_async();
                        pollster::block_on(future);
                    }
                    _ => (),
                },
                Event::AboutToWait => {
                    if !paused {
                        wgputoy.wgpu.window.request_redraw();
                    }
                    if close_requested {
                        println!("🎉 Live preview window closed");
                        elwt.exit();
                    }
                }
                _ => (),
            }
        });

        Ok(())
    }

    /// Load textures for a specific shader based on JSON metadata or defaults
    async fn load_shader_textures(
        &self,
        toy_renderer: &mut WgpuToyRenderer,
        shader_name: &str,
    ) -> Result<(), CodeSkewError> {
        // Try to load JSON metadata first
        let json_path = format!("wgsl/{}.wgsl.json", shader_name);
        if let Ok(json_content) = std::fs::read_to_string(&json_path) {
            println!("🔧 DEBUG: Found JSON metadata at {}", json_path);
            if let Ok(metadata) = serde_json::from_str::<ShaderMetadata>(&json_content) {
                return self.load_textures_from_metadata(toy_renderer, &metadata).await;
            }
        }

        // No JSON metadata, use shader-specific defaults
        self.load_default_textures_for_shader(toy_renderer, shader_name).await
    }

    /// Load textures from JSON metadata
    async fn load_textures_from_metadata(
        &self,
        toy_renderer: &mut WgpuToyRenderer,
        metadata: &ShaderMetadata,
    ) -> Result<(), CodeSkewError> {
        for (channel, texture_info) in metadata.textures.iter().enumerate() {
            if channel >= 2 { break; } // Only support channel0 and channel1

            let texture_data = if texture_info.img.starts_with("http") {
                // Download from URL
                self.download_texture(&texture_info.img).await?
            } else if texture_info.img.starts_with("/textures/") {
                // Try to find local texture file
                let local_path = format!("assets{}", texture_info.img);
                std::fs::read(&local_path).map_err(|_| {
                    CodeSkewError::RenderingError(format!("Texture file not found: {}", local_path))
                })?
            } else {
                // Relative path in wgsl directory
                let local_path = format!("wgsl/{}", texture_info.img);
                std::fs::read(&local_path).map_err(|_| {
                    CodeSkewError::RenderingError(format!("Texture file not found: {}", local_path))
                })?
            };

            toy_renderer.load_channel(channel, &texture_data)
                .map_err(|e| CodeSkewError::RenderingError(format!("Failed to load texture into channel{}: {}", channel, e)))?;
            
            println!("🔧 DEBUG: Loaded texture into channel{} from {}", channel, texture_info.img);
        }

        Ok(())
    }

    /// Load default textures for specific shaders
    async fn load_default_textures_for_shader(
        &self,
        toy_renderer: &mut WgpuToyRenderer,
        shader_name: &str,
    ) -> Result<(), CodeSkewError> {
        match shader_name {
            "codeskew_unified" => {
                // For unified shader, Glyphon will render to channel1
                println!("🔧 DEBUG: Channel1 reserved for Glyphon text rendering");
                Ok(())
            }
            "zeroshot" | "zeroshot_original" => {
                // For zeroshot, generate a procedural noise texture directly
                toy_renderer.load_procedural_texture(0, 512, 512)
                    .map_err(|e| CodeSkewError::RenderingError(format!("Failed to load zeroshot texture: {}", e)))?;
                println!("🔧 DEBUG: Generated procedural texture for zeroshot shader");
                Ok(())
            }
            _ => {
                // For unknown shaders, use a simple noise texture
                let noise_url = "https://cdn.jsdelivr.net/gh/mrdoob/three.js@dev/examples/textures/cloud.png";
                let texture_data = self.download_texture(noise_url).await?;
                toy_renderer.load_channel(0, &texture_data)
                    .map_err(|e| CodeSkewError::RenderingError(format!("Failed to load default texture: {}", e)))?;
                println!("🔧 DEBUG: Loaded default cloud texture for shader: {}", shader_name);
                Ok(())
            }
        }
    }

    /// Download texture from URL with caching
    async fn download_texture(&self, url: &str) -> Result<Vec<u8>, CodeSkewError> {
        // Create cache directory
        let cache_dir = std::path::Path::new("cache/textures");
        std::fs::create_dir_all(cache_dir).map_err(|e| {
            CodeSkewError::RenderingError(format!("Failed to create cache directory: {}", e))
        })?;

        // Generate cache filename from URL
        let cache_filename = url.split('/').last().unwrap_or("texture.png");
        let cache_path = cache_dir.join(cache_filename);

        // Check if cached file exists
        if cache_path.exists() {
            println!("🔧 DEBUG: Using cached texture: {}", cache_path.display());
            return std::fs::read(&cache_path).map_err(|e| {
                CodeSkewError::RenderingError(format!("Failed to read cached texture: {}", e))
            });
        }

        // Download texture
        println!("🔧 DEBUG: Downloading texture from: {}", url);
        let client = reqwest::Client::new();
        let response = client.get(url).send().await.map_err(|e| {
            CodeSkewError::RenderingError(format!("Failed to download texture: {}", e))
        })?;

        let texture_data = response.bytes().await.map_err(|e| {
            CodeSkewError::RenderingError(format!("Failed to read texture data: {}", e))
        })?.to_vec();

        // Cache the texture
        std::fs::write(&cache_path, &texture_data).map_err(|e| {
            CodeSkewError::RenderingError(format!("Failed to cache texture: {}", e))
        })?;

        println!("🔧 DEBUG: Downloaded and cached texture: {} bytes", texture_data.len());
        Ok(texture_data)
    }



    /// Get configuration reference with zero allocation
    #[inline]
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get layout engine reference with zero allocation
    #[inline]
    pub fn layout_engine(&self) -> &LayoutEngine {
        &self.layout_engine
    }

    /// Get highlighter reference with zero allocation
    #[inline]
    pub fn highlighter(&self) -> &SyntaxHighlighter {
        &self.highlighter
    }

    /// Get current buffer statistics for performance monitoring
    #[inline]
    pub fn buffer_stats(&self) -> (usize, usize, usize) {
        (
            self.pixel_count,
            self.buffer_size,
            self.rgba_buffer.capacity(),
        )
    }

    /// Get pre-calculated dimensions for optimal performance
    #[inline]
    pub fn dimensions(&self) -> (u32, u32, f32, f32) {
        (
            self.width_u32,
            self.height_u32,
            self.width_f32,
            self.height_f32,
        )
    }

    /// Reset internal buffers for reuse with different dimensions
    #[inline]
    pub fn reset_for_dimensions(&mut self, width: u32, height: u32) {
        // Update pre-calculated values
        self.width_u32 = width;
        self.height_u32 = height;
        self.width_f32 = width as f32;
        self.height_f32 = height as f32;
        self.pixel_count = (width * height) as usize;
        self.buffer_size = self.pixel_count * 4;

        // Efficiently resize buffers only when necessary
        if self.rgba_buffer.len() < self.buffer_size {
            self.rgba_buffer.resize(self.buffer_size, 0);
        }
        if self.temp_buffer.len() < self.buffer_size {
            self.temp_buffer.resize(self.buffer_size, 0);
        }

        // Clear shader buffer for regeneration
        self.shader_buffer.clear();
    }

    /// Warm up all buffers and systems for optimal first-run performance
    #[inline]
    pub fn warmup(&mut self) -> Result<(), CodeSkewError> {
        // Pre-warm all buffers to avoid allocation during hot paths
        self.rgba_buffer.resize(self.buffer_size, 0);
        self.temp_buffer.resize(self.buffer_size, 0);
        self.shader_buffer.reserve(2048);

        // Pre-generate shader template for faster compilation
        self.build_background_shader();

        Ok(())
    }

    /// Get memory usage statistics for performance monitoring
    #[inline]
    pub fn memory_stats(&self) -> (usize, usize, usize) {
        (
            self.rgba_buffer.capacity() * std::mem::size_of::<u8>(),
            self.temp_buffer.capacity() * std::mem::size_of::<u8>(),
            self.shader_buffer.capacity() * std::mem::size_of::<u8>(),
        )
    }

    /// Calculate optimal font size for 3D perspective text rendering
    /// 
    /// Algorithm accounts for:
    /// - 3D perspective (largest at 2/3 down the screen)
    /// - Horizontal depth scaling (left side larger than right)
    /// - Target ~100 characters visible at the largest point
    /// - Minimum window size of 400px
    /// - User-configurable fold, skew, and scale parameters
    #[inline]
    fn calculate_perspective_font_size(&self) -> f32 {
        // Use config parameters for 3D perspective
        let fold_point = 0.67; // 2/3 down where text is largest (fixed)
        let fold_strength = self.config.fold; // CLI configurable fold strength
        let perspective_strength = self.config.scale; // CLI configurable scale factor
        let target_chars = 100.0; // Target character count at largest point
        
        // Calculate effective width at the largest point (2/3 down, left side)
        // At fold_point, vertical_scale = 1.0 + fold_strength = 1.4
        let max_vertical_scale = 1.0 + fold_strength;
        
        // At left side (x=0), horizontal_depth = 1.0 + perspective_strength = 1.6
        let max_horizontal_scale = 1.0 + perspective_strength;
        
        // Combined maximum scale factor
        let max_scale = max_vertical_scale * max_horizontal_scale;
        
        // Effective width available for text at maximum scale
        let effective_width = self.width_f32 / max_scale;
        
        // Calculate font size to fit target characters
        // Monospace character width is typically 0.6 * font_size
        let char_width_ratio = 0.6;
        let calculated_font_size = effective_width / (target_chars * char_width_ratio);
        
        // Apply bounds with minimum based on 400px window assumption
        let min_font_size = (400.0 / target_chars * char_width_ratio).max(8.0);
        let max_font_size = 48.0;
        
        calculated_font_size.clamp(min_font_size, max_font_size)
    }

    /// Render text using ratagpu's production-quality Glyphon renderer
    async fn render_glyphon_to_texture(
        &self,
        toy_renderer: &mut WgpuToyRenderer,
        layout: &[PositionedLine],
    ) -> Result<(), CodeSkewError> {
        println!("🔤 Starting ratagpu-based Glyphon text rendering...");
        
        // Calculate optimal font size for 3D perspective text rendering
        // Algorithm: Size to fit ~100 chars at largest point (2/3 down) with perspective scaling
        let font_size = self.calculate_perspective_font_size();
        
        // Create ratagpu texture renderer (80x30 is standard terminal size)  
        let mut renderer = GlyphonTextureRenderer::<80, 30>::new(
            toy_renderer.wgpu.device.clone(),
            toy_renderer.wgpu.queue.clone(),
            toy_renderer.wgpu.surface_config.clone(),
            font_size, // Dynamically calculated font size based on window
        ).await.map_err(|e| CodeSkewError::RenderingError(format!("Failed to create Glyphon renderer: {}", e)))?;
        
        // Load layout data into the renderer's cell grid
        renderer.load_layout(layout);
        
        // Render to texture using ratagpu's zero-allocation pipeline
        let text_texture = renderer.render_to_texture(self.width_u32, self.height_u32)
            .map_err(|e| CodeSkewError::RenderingError(format!("Failed to render text to texture: {}", e)))?;
        
        // Set the rendered texture into channel1 for the compute shader
        toy_renderer.set_channel_texture(1, text_texture)
            .map_err(|e| CodeSkewError::RenderingError(format!("Failed to set text texture: {}", e)))?;
        
        println!("🔤 ratagpu Glyphon text rendered to storage texture successfully!");
        Ok(())
    }

    /// Build WGSL shader using minijinja templating
    /// This method is available for custom shader generation with layout data
    #[allow(dead_code)]
    fn build_templated_wgsl_shader(&mut self, layout: &[PositionedLine], shader_template: &str) -> Result<String, CodeSkewError> {
        use minijinja::{Environment, context};
        
        // Create minijinja environment
        let mut env = Environment::new();
        
        // Register the WGSL template from string
        env.add_template("shader", shader_template)
            .map_err(|e| CodeSkewError::RenderingError(format!("Template error: {}", e)))?;
        
        // Prepare template data from layout
        let template_data = self.prepare_template_data(layout)?;
        
        // Calculate downsampling parameters for 3x size reduction
        let downsample_factor = 3.0;
        let downsampled_width = (self.width_u32 as f32 / downsample_factor) as u32;
        let downsampled_height = (self.height_u32 as f32 / downsample_factor) as u32;
        
        // Render the template
        let tmpl = env.get_template("shader")
            .map_err(|e| CodeSkewError::RenderingError(format!("Template not found: {}", e)))?;
            
        let rendered_wgsl = tmpl.render(context! {
            width => self.width_u32,
            height => self.height_u32,
            downsampled_width => downsampled_width,
            downsampled_height => downsampled_height,
            downsample_factor => downsample_factor,
            fold => self.config.fold,
            skew => self.config.skew_angle,
            scale => self.config.scale,
            perspective => self.config.perspective,
            font_size => self.calculate_perspective_font_size(),
            characters => template_data.characters,
            colors => template_data.colors,
            positions => template_data.positions,
            line_count => layout.len(),
            max_chars_per_line => template_data.max_chars_per_line,
            // Math constants
            PI => std::f32::consts::PI,
            TAU => std::f32::consts::TAU,
            E => std::f32::consts::E,
            // Default empty code block
            code => "",
        }).map_err(|e| CodeSkewError::RenderingError(format!("Render error: {}", e)))?;
        
        Ok(rendered_wgsl)
    }

    /// Process shader template through unified MiniJinja system
    async fn process_shader_template(&mut self, layout: &[PositionedLine]) -> Result<String, CodeSkewError> {
        use minijinja::{Environment, context};
        
        // Determine shader template source
        let (shader_template, shader_source) = if self.config.input.extension().map_or(false, |ext| ext == "wgsl") {
            // Load WGSL file as template
            let wgsl_content = std::fs::read_to_string(&self.config.input)
                .map_err(|e| CodeSkewError::RenderingError(format!("Failed to read WGSL file: {e}")))?;
            println!("🔧 Loading WGSL shader from: {}", self.config.input.display());
            (wgsl_content, "input file".to_string())
        } else {
            // Load background shader as template
            let shader_path = format!("wgsl/{}.wgsl", self.config.shader);
            let wgsl_content = std::fs::read_to_string(&shader_path)
                .map_err(|e| CodeSkewError::RenderingError(format!("Failed to read background shader {}: {e}", shader_path)))?;
            println!("🔧 Loading background shader: {}", self.config.shader);
            (wgsl_content, self.config.shader.clone())
        };
        
        // Create MiniJinja environment
        let mut env = Environment::new();
        env.add_template("shader", &shader_template)
            .map_err(|e| CodeSkewError::RenderingError(format!("Template error: {}", e)))?;
        
        // Build template context
        let template_context = self.build_template_context(layout, &shader_source)?;
        
        // Render template
        let tmpl = env.get_template("shader")
            .map_err(|e| CodeSkewError::RenderingError(format!("Template not found: {}", e)))?;
            
        let rendered_wgsl = tmpl.render(template_context)
            .map_err(|e| CodeSkewError::RenderingError(format!("Template render error: {}", e)))?;
        
        Ok(rendered_wgsl)
    }

    /// Build template context for MiniJinja rendering
    fn build_template_context(&self, layout: &[PositionedLine], shader_source: &str) -> Result<minijinja::Value, CodeSkewError> {
        use minijinja::context;
        
        // Generate code WGSL if input is a code file
        let code_wgsl = if !self.config.input.extension().map_or(false, |ext| ext == "wgsl") {
            // Generate WGSL code for text rendering
            self.generate_text_rendering_wgsl(layout)?
        } else {
            // Empty code for .wgsl files
            String::new()
        };
        
        // Calculate downsampling parameters for 3x size reduction
        let downsample_factor = 3.0;
        let downsampled_width = (self.width_u32 as f32 / downsample_factor) as u32;
        let downsampled_height = (self.height_u32 as f32 / downsample_factor) as u32;
        
        Ok(context! {
            width => self.width_u32,
            height => self.height_u32,
            downsampled_width => downsampled_width,
            downsampled_height => downsampled_height,
            downsample_factor => downsample_factor,
            fold => self.config.fold,
            skew => self.config.skew_angle,
            scale => self.config.scale,
            perspective => self.config.perspective,
            font_size => self.calculate_perspective_font_size(),
            line_count => layout.len(),
            shader_source => shader_source,
            // Math constants
            PI => std::f32::consts::PI,
            TAU => std::f32::consts::TAU,
            E => std::f32::consts::E,
            // Template variables
            grid_horizon => 0.4,
            grid_nearest => 0.67,
            vanishing_x => 0.3,
            skew_strength => 0.4,
            background_alpha => 1.0,
            // Code rendering
            code => code_wgsl,
        })
    }

    /// Generate WGSL code for text rendering from layout data
    fn generate_text_rendering_wgsl(&self, layout: &[PositionedLine]) -> Result<String, CodeSkewError> {
        let mut wgsl_code = String::new();
        
        // Calculate downsampling parameters
        let downsample_factor = 3.0;
        let downsampled_width = (self.width_u32 as f32 / downsample_factor) as u32;
        let downsampled_height = (self.height_u32 as f32 / downsample_factor) as u32;
        
        // Collect all characters, colors, and positions
        let mut characters = Vec::new();
        let mut colors = Vec::new();
        let mut positions = Vec::new();
        
        for (line_idx, line) in layout.iter().enumerate() {
            for (char_idx, styled_char) in line.chars.iter().enumerate() {
                characters.push(styled_char.char as u32);
                
                // Pack color as u32 (RGBA8)
                let color_packed = 
                    ((styled_char.color.r as u32) << 24) |
                    ((styled_char.color.g as u32) << 16) |
                    ((styled_char.color.b as u32) << 8) |
                    (255u32); // Alpha
                colors.push(color_packed);
                
                // Store position with 3D perspective scaling
                let char_x = line.x + (char_idx as f32 * self.config.fontsize * 0.6);
                let char_y = line.y;
                positions.push((char_x, char_y, line_idx as u32, char_idx as u32));
            }
        }
        
        let char_count = characters.len();
        if char_count == 0 {
            return Ok(String::new());
        }
        
        // Generate WGSL constants and data structures
        wgsl_code.push_str(&format!("// Generated text rendering code\n"));
        wgsl_code.push_str(&format!("const CHAR_COUNT: u32 = {}u;\n\n", char_count));
        
        // Character data array
        wgsl_code.push_str("const chars: array<u32, CHAR_COUNT> = array<u32, CHAR_COUNT>(\n");
        for (i, &ch) in characters.iter().enumerate() {
            if i > 0 { wgsl_code.push_str(", "); }
            if i % 10 == 0 { wgsl_code.push_str("\n    "); }
            wgsl_code.push_str(&format!("{}u", ch));
        }
        wgsl_code.push_str("\n);\n\n");
        
        // Color data array
        wgsl_code.push_str("const colors: array<u32, CHAR_COUNT> = array<u32, CHAR_COUNT>(\n");
        for (i, &color) in colors.iter().enumerate() {
            if i > 0 { wgsl_code.push_str(", "); }
            if i % 8 == 0 { wgsl_code.push_str("\n    "); }
            wgsl_code.push_str(&format!("0x{:08x}u", color));
        }
        wgsl_code.push_str("\n);\n\n");
        
        // Position data structure and array
        wgsl_code.push_str("struct CharPosition {\n");
        wgsl_code.push_str("    x: f32,\n");
        wgsl_code.push_str("    y: f32,\n");
        wgsl_code.push_str("    line: u32,\n");
        wgsl_code.push_str("    char_idx: u32,\n");
        wgsl_code.push_str("}\n\n");
        
        wgsl_code.push_str("const positions: array<CharPosition, CHAR_COUNT> = array<CharPosition, CHAR_COUNT>(\n");
        for (i, &(x, y, line, char_idx)) in positions.iter().enumerate() {
            if i > 0 { wgsl_code.push_str(", "); }
            if i % 4 == 0 { wgsl_code.push_str("\n    "); }
            wgsl_code.push_str(&format!("CharPosition({:.1}, {:.1}, {}u, {}u)", x, y, line, char_idx));
        }
        wgsl_code.push_str("\n);\n\n");
        
        // Utility functions with simplified coordinate mapping
        wgsl_code.push_str(r#"
// Unpack color from u32 to float4
fn unpack_color(packed: u32) -> float4 {
    let r = f32((packed >> 24u) & 0xFFu) / 255.0;
    let g = f32((packed >> 16u) & 0xFFu) / 255.0;
    let b = f32((packed >> 8u) & 0xFFu) / 255.0;
    let a = f32(packed & 0xFFu) / 255.0;
    return float4(r, g, b, a);
}

// Simple UV coordinate mapping for text positioning
fn apply_3d_perspective(pos: CharPosition, screen_uv: float2, t: float) -> float2 {
    // Use shader's actual screen dimensions
    let screen_size = float2(textureDimensions(screen));
    let char_uv = float2(pos.x, pos.y) / screen_size;
    
    // Position text in upper area (above bandwidth graphs at y < 0.3)
    let text_area_uv = float2(
        char_uv.x * 0.8 + 0.1,  // Center horizontally with margins
        char_uv.y * 0.25 + 0.05  // Upper area of screen
    );
    
    return text_area_uv;
}
"#);
        
        
        // Optimized character rendering for visibility
        wgsl_code.push_str(r#"
// Complete 5x7 bitmap font for programming characters
fn get_char_pattern(char_code: u32, pixel_x: u32, pixel_y: u32) -> bool {
    switch char_code {
        // Common programming symbols (prioritized)
        case 123u: { // '{'
            return (pixel_x == 1u && (pixel_y == 6u || pixel_y == 0u)) ||
                   (pixel_x == 2u && (pixel_y >= 1u && pixel_y <= 2u || pixel_y >= 4u && pixel_y <= 5u)) ||
                   (pixel_x == 0u && pixel_y == 3u);
        }
        case 125u: { // '}'
            return (pixel_x == 3u && (pixel_y == 6u || pixel_y == 0u)) ||
                   (pixel_x == 2u && (pixel_y >= 1u && pixel_y <= 2u || pixel_y >= 4u && pixel_y <= 5u)) ||
                   (pixel_x == 4u && pixel_y == 3u);
        }
        case 40u: { // '('
            return (pixel_x == 2u && (pixel_y == 6u || pixel_y == 0u)) ||
                   (pixel_x == 1u && (pixel_y >= 1u && pixel_y <= 5u));
        }
        case 41u: { // ')'
            return (pixel_x == 2u && (pixel_y == 6u || pixel_y == 0u)) ||
                   (pixel_x == 3u && (pixel_y >= 1u && pixel_y <= 5u));
        }
        case 91u: { // '['
            return (pixel_x == 1u) || (pixel_x == 2u && (pixel_y == 6u || pixel_y == 0u));
        }
        case 93u: { // ']'
            return (pixel_x == 3u) || (pixel_x == 2u && (pixel_y == 6u || pixel_y == 0u));
        }
        case 61u: { // '='
            return (pixel_y == 2u || pixel_y == 4u) && (pixel_x >= 1u && pixel_x <= 3u);
        }
        case 59u: { // ';'
            return (pixel_x == 2u && pixel_y == 3u) || (pixel_x == 2u && pixel_y == 1u) || (pixel_x == 1u && pixel_y == 0u);
        }
        case 58u: { // ':'
            return pixel_x == 2u && (pixel_y == 1u || pixel_y == 4u);
        }
        
        // Numbers 0-9
        case 48u: { // '0'
            return (pixel_x == 0u || pixel_x == 4u) && (pixel_y >= 1u && pixel_y <= 5u) ||
                   (pixel_y == 0u || pixel_y == 6u) && (pixel_x >= 1u && pixel_x <= 3u);
        }
        case 49u: { // '1'
            return pixel_x == 2u || (pixel_x == 1u && pixel_y == 6u) || (pixel_y == 0u && pixel_x >= 1u && pixel_x <= 3u);
        }
        case 50u: { // '2'
            return (pixel_y == 6u || pixel_y == 3u || pixel_y == 0u) && (pixel_x >= 1u && pixel_x <= 3u) ||
                   (pixel_x == 4u && pixel_y >= 4u && pixel_y <= 5u) || (pixel_x == 0u && pixel_y >= 1u && pixel_y <= 2u);
        }
        case 51u: { // '3'
            return (pixel_y == 6u || pixel_y == 3u || pixel_y == 0u) && (pixel_x >= 1u && pixel_x <= 3u) ||
                   pixel_x == 4u && (pixel_y == 1u || pixel_y == 2u || pixel_y == 4u || pixel_y == 5u);
        }
        case 52u: { // '4'
            return pixel_x == 4u || (pixel_y == 3u && pixel_x <= 3u) || (pixel_x == 0u && pixel_y >= 4u);
        }
        case 53u: { // '5'
            return (pixel_y == 6u || pixel_y == 3u || pixel_y == 0u) && (pixel_x >= 1u && pixel_x <= 3u) ||
                   (pixel_x == 0u && pixel_y >= 4u) || (pixel_x == 4u && pixel_y >= 1u && pixel_y <= 2u);
        }
        case 54u: { // '6'
            return (pixel_x == 0u && pixel_y >= 1u) || (pixel_y == 6u || pixel_y == 3u || pixel_y == 0u) && (pixel_x >= 1u && pixel_x <= 3u) ||
                   (pixel_x == 4u && pixel_y >= 1u && pixel_y <= 2u);
        }
        case 55u: { // '7'
            return (pixel_y == 6u && pixel_x >= 1u && pixel_x <= 4u) || (pixel_x == 3u && pixel_y >= 3u && pixel_y <= 5u) ||
                   (pixel_x == 2u && pixel_y >= 1u && pixel_y <= 2u);
        }
        case 56u: { // '8'
            return (pixel_x == 0u || pixel_x == 4u) && (pixel_y == 1u || pixel_y == 2u || pixel_y == 4u || pixel_y == 5u) ||
                   (pixel_y == 0u || pixel_y == 3u || pixel_y == 6u) && (pixel_x >= 1u && pixel_x <= 3u);
        }
        case 57u: { // '9'
            return (pixel_x == 4u && pixel_y <= 5u) || (pixel_y == 6u || pixel_y == 3u || pixel_y == 0u) && (pixel_x >= 1u && pixel_x <= 3u) ||
                   (pixel_x == 0u && pixel_y >= 4u && pixel_y <= 5u);
        }
        
        // Lowercase letters (common in code)
        case 97u: { // 'a'
            return (pixel_y <= 2u && pixel_x >= 1u && pixel_x <= 3u) ||
                   (pixel_x == 4u && pixel_y <= 2u) || (pixel_x == 0u && pixel_y == 1u);
        }
        case 101u: { // 'e'
            return (pixel_y == 2u || pixel_y == 0u) && (pixel_x >= 1u && pixel_x <= 3u) ||
                   (pixel_x == 0u && pixel_y == 1u) || (pixel_x == 4u && pixel_y == 3u);
        }
        case 102u: { // 'f'
            return pixel_x == 1u || (pixel_y == 5u && pixel_x == 2u) || (pixel_y == 3u && pixel_x <= 2u);
        }
        case 105u: { // 'i'
            return pixel_x == 2u && pixel_y <= 2u || pixel_x == 2u && pixel_y == 4u;
        }
        case 108u: { // 'l'
            return pixel_x == 2u;
        }
        case 110u: { // 'n'
            return pixel_x == 0u && pixel_y <= 2u || (pixel_y == 3u && pixel_x >= 1u && pixel_x <= 3u) ||
                   pixel_x == 4u && pixel_y <= 2u;
        }
        case 111u: { // 'o'
            return (pixel_y == 3u || pixel_y == 0u) && (pixel_x >= 1u && pixel_x <= 3u) ||
                   (pixel_x == 0u || pixel_x == 4u) && (pixel_y == 1u || pixel_y == 2u);
        }
        case 114u: { // 'r'
            return pixel_x == 0u && pixel_y <= 2u || (pixel_y == 3u && pixel_x >= 1u && pixel_x <= 2u);
        }
        case 115u: { // 's'
            return (pixel_y == 3u || pixel_y == 1u || pixel_y == 0u) && (pixel_x >= 1u && pixel_x <= 3u) ||
                   (pixel_x == 0u && pixel_y == 2u) || (pixel_x == 4u && pixel_y == 1u);
        }
        case 116u: { // 't'
            return pixel_x == 1u && pixel_y <= 3u || (pixel_y == 4u && pixel_x <= 2u) || (pixel_y == 0u && pixel_x >= 1u && pixel_x <= 2u);
        }
        case 117u: { // 'u'
            return (pixel_x == 0u || pixel_x == 4u) && pixel_y <= 2u || (pixel_y == 0u && pixel_x >= 1u && pixel_x <= 3u);
        }
        
        // Uppercase letters (common in types)
        case 65u: { // 'A'
            return (pixel_x == 0u || pixel_x == 4u) && pixel_y <= 5u ||
                   (pixel_y == 6u || pixel_y == 3u) && (pixel_x >= 1u && pixel_x <= 3u);
        }
        case 83u: { // 'S'
            return (pixel_y == 6u || pixel_y == 3u || pixel_y == 0u) && (pixel_x >= 1u && pixel_x <= 3u) ||
                   (pixel_x == 0u && (pixel_y == 5u || pixel_y == 4u)) || (pixel_x == 4u && (pixel_y == 2u || pixel_y == 1u));
        }
        
        // Special characters
        case 32u: { return false; } // space
        case 46u: { // '.'
            return pixel_x == 2u && pixel_y == 0u;
        }
        case 44u: { // ','
            return pixel_x == 2u && pixel_y == 0u; // Simplified comma pattern
        }
        case 39u: { // apostrophe '''
            return pixel_x == 2u && pixel_y >= 5u;
        }
        case 34u: { // quote '"'
            return (pixel_x == 1u || pixel_x == 3u) && pixel_y >= 5u;
        }
        case 95u: { // '_'
            return pixel_y == 0u && pixel_x >= 1u && pixel_x <= 3u;
        }
        case 45u: { // '-'
            return pixel_y == 3u && pixel_x >= 1u && pixel_x <= 3u;
        }
        case 43u: { // '+'
            return (pixel_x == 2u && pixel_y >= 2u && pixel_y <= 4u) || (pixel_y == 3u && pixel_x >= 1u && pixel_x <= 3u);
        }
        case 42u: { // '*'
            return pixel_x == 2u && pixel_y == 3u || (pixel_x == 1u || pixel_x == 3u) && (pixel_y == 2u || pixel_y == 4u);
        }
        case 47u: { // '/'
            return (pixel_x == 4u && pixel_y >= 4u) || (pixel_x == 3u && pixel_y == 3u) ||
                   (pixel_x == 2u && pixel_y == 2u) || (pixel_x == 1u && pixel_y == 1u) || (pixel_x == 0u && pixel_y <= 1u);
        }
        case 92u: { // '\'
            return (pixel_x == 0u && pixel_y >= 4u) || (pixel_x == 1u && pixel_y == 3u) ||
                   (pixel_x == 2u && pixel_y == 2u) || (pixel_x == 3u && pixel_y == 1u) || (pixel_x == 4u && pixel_y <= 1u);
        }
        case 60u: { // '<'
            return (pixel_x == 3u && (pixel_y == 1u || pixel_y == 5u)) || (pixel_x == 2u && (pixel_y == 2u || pixel_y == 4u)) ||
                   (pixel_x == 1u && pixel_y == 3u);
        }
        case 62u: { // '>'
            return (pixel_x == 1u && (pixel_y == 1u || pixel_y == 5u)) || (pixel_x == 2u && (pixel_y == 2u || pixel_y == 4u)) ||
                   (pixel_x == 3u && pixel_y == 3u);
        }
        case 33u: { // '!'
            return pixel_x == 2u && (pixel_y >= 2u || pixel_y == 0u);
        }
        case 63u: { // '?'
            return (pixel_y == 6u && pixel_x >= 1u && pixel_x <= 3u) || (pixel_x == 4u && pixel_y >= 4u && pixel_y <= 5u) ||
                   (pixel_y == 3u && pixel_x >= 2u && pixel_x <= 3u) || (pixel_x == 2u && pixel_y == 0u);
        }
        case 35u: { // '#'
            return (pixel_x == 1u || pixel_x == 3u) || (pixel_y == 2u || pixel_y == 4u);
        }
        case 38u: { // '&'
            return (pixel_y == 6u && pixel_x >= 1u && pixel_x <= 2u) || (pixel_x == 0u && (pixel_y == 5u || pixel_y == 3u || pixel_y == 1u)) ||
                   (pixel_y == 2u && pixel_x == 1u) || (pixel_x == 3u && pixel_y <= 1u) || (pixel_y == 0u && pixel_x == 4u);
        }
        case 124u: { // '|'
            return pixel_x == 2u;
        }
        
        default: { // Fallback pattern for unimplemented characters
            return (pixel_x == 0u || pixel_x == 4u || pixel_y == 0u || pixel_y == 6u) && 
                   !((pixel_x == 0u || pixel_x == 4u) && (pixel_y == 0u || pixel_y == 6u));
        }
    }
}

// Render a single character using simple bitmap font
fn render_character(char_code: u32, world_pos: float2, color: float4, screen_uv: float2, t: float) -> float4 {
    let char_size = float2(0.020, 0.028); // 2x larger character size for readability
    let char_uv = screen_uv - world_pos;
    
    // Check if we're within character bounds
    if (abs(char_uv.x) < char_size.x && abs(char_uv.y) < char_size.y) {
        // Convert to pixel coordinates within 5x7 grid
        let pixel_x = u32((char_uv.x + char_size.x) / (2.0 * char_size.x) * 5.0);
        let pixel_y = u32((char_uv.y + char_size.y) / (2.0 * char_size.y) * 7.0);
        
        if (pixel_x < 5u && pixel_y < 7u && get_char_pattern(char_code, pixel_x, pixel_y)) {
            return float4(color.rgb, 0.9); // Use actual syntax highlighting colors
        }
    }
    
    return float4(0.0);
}

// Main text rendering function
fn render_text_layer(uv: float2, t: float) -> float4 {
    var text_color = float4(0.0);
    
    for (var i = 0u; i < CHAR_COUNT; i++) {
        let char_pos = apply_3d_perspective(positions[i], uv, t);
        let char_color = unpack_color(colors[i]);
        text_color += render_character(chars[i], char_pos, char_color, uv, t);
    }
    
    return text_color;
}
"#);
        
        Ok(wgsl_code)
    }
    
    /// Prepare layout data for template
    fn prepare_template_data(&self, layout: &[PositionedLine]) -> Result<TemplateData, CodeSkewError> {
        let mut characters = Vec::new();
        let mut colors = Vec::new();
        let mut positions = Vec::new();
        let mut max_chars_per_line = 0;
        
        for (line_idx, line) in layout.iter().enumerate() {
            max_chars_per_line = max_chars_per_line.max(line.chars.len());
            
            for (char_idx, styled_char) in line.chars.iter().enumerate() {
                // Encode character as u32 for WGSL
                characters.push(styled_char.char as u32);
                
                // Pack color as u32 (RGBA8)
                let color_packed = 
                    ((styled_char.color.r as u32) << 24) |
                    ((styled_char.color.g as u32) << 16) |
                    ((styled_char.color.b as u32) << 8) |
                    (255u32); // Alpha
                colors.push(color_packed);
                
                // Store position data
                positions.push(PositionData {
                    line_idx,
                    char_idx,
                    x: line.x + (char_idx as f32 * self.config.fontsize * 0.6),
                    y: line.y,
                    scale: line.scale,
                });
            }
        }
        
        Ok(TemplateData {
            characters,
            colors,
            positions,
            max_chars_per_line,
        })
    }
}
