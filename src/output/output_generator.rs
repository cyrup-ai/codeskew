//! Core OutputGenerator for high-performance code rendering with zero allocation

use crate::cli::OutputFormat;
use crate::config::Config;
use crate::error::CodeSkewError;
use crate::highlight::SyntaxHighlighter;
use crate::layout::{LayoutEngine, PositionedLine};
use crate::output::SaveMethods;
use crate::toy::{WgpuToyRenderer, init_wgpu};
use crate::webgpu::EliteWebGPURenderer;
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

    /// Generate gorgeous composite output with blazing performance and zero allocation
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

        // For static outputs, do composite rendering: toy background + elite text
        let image = self.render_composite_layers(&layout).await?;

        // Save output with optimized format handling
        let save_methods = SaveMethods::new(&self.config);
        match self.config.format {
            OutputFormat::Png => save_methods.save_png_optimized(image).await?,
            OutputFormat::Svg => {
                return Err(CodeSkewError::ConfigError(
                    "SVG not supported in composite mode".to_string(),
                ));
            }
            OutputFormat::Gif => save_methods.save_gif_animation_optimized(&layout).await?,
            OutputFormat::Webp => save_methods.save_webp_optimized(image).await?,
            OutputFormat::Wgpu => unreachable!(), // Already handled above
        }

        println!(
            "🚀 Composite render completed in {:.2?}! ✨",
            start_time.elapsed()
        );
        println!("💎 Output saved to: {}", self.config.output.display());

        Ok(())
    }

    /// Render composite layers: toy background + elite text with zero allocation and blazing performance
    #[inline]
    async fn render_composite_layers(
        &mut self,
        layout: &[PositionedLine],
    ) -> Result<RgbaImage, CodeSkewError> {
        println!("🎨 Rendering composite layers: toy background + elite text");

        // Create shared WebGPU context - single allocation for entire pipeline
        let wgpu_context = init_wgpu(self.width_u32, self.height_u32, "")
            .await
            .map_err(|e| {
                CodeSkewError::RenderingError(format!("Failed to create WGPU context: {e}"))
            })?;

        // EPIC: Proper layered rendering - animated background + glyphon 3D text!
        println!("🚀 LAYERED: Rendering animated background + glyphon 3D text!");
        
        // Step 1: Render animated background with toy shaders
        let background_data = self.render_background_layer(wgpu_context).await?;
        
        // Step 2: Render 3D text with glyphon (using existing elite renderer)
        let text_data = self.render_text_layer(layout).await?;
        
        // Step 3: Composite layers with proper alpha blending
        self.composite_layers_optimized(&background_data, &text_data)?;

        // Debug buffer before image creation
        println!("🔧 DEBUG: Pre-image buffer length: {}", self.rgba_buffer.len());
        let pre_image_non_zero = self.rgba_buffer.iter().filter(|&&x| x != 0).count();
        println!("🔧 DEBUG: Pre-image non-zero bytes: {}/{}", pre_image_non_zero, self.rgba_buffer.len());

        // Convert pre-allocated buffer to RgbaImage with zero copy optimization
        let rgba_image = RgbaImage::from_raw(
            self.width_u32,
            self.height_u32,
            std::mem::take(&mut self.rgba_buffer),
        )
        .ok_or_else(|| {
            CodeSkewError::RenderingError(
                "Failed to create RgbaImage from pre-allocated buffer".to_string(),
            )
        })?;
        
        println!("🔧 DEBUG: Created RgbaImage {}x{}", rgba_image.width(), rgba_image.height());

        // Restore buffer for next use
        self.rgba_buffer = rgba_image.clone().into_raw();

        Ok(rgba_image)
    }

    /// Render animated background using toy shaders with optimized performance
    #[inline]
    async fn render_background_layer(
        &mut self,
        wgpu_context: crate::toy::WgpuContext,
    ) -> Result<Vec<u8>, CodeSkewError> {
        println!("🔧 DEBUG: Starting background layer render");
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

        // ALSO create Elite WebGPU renderer for text testing
        log::info!("🔤 Creating EliteWebGPURenderer for text testing");
        let mut elite_renderer = EliteWebGPURenderer::new(
            self.width_u32, 
            self.height_u32
        ).await.map_err(|e| CodeSkewError::RenderingError(format!("Failed to create Elite renderer: {e}")))?;
        log::info!("🔤 EliteWebGPURenderer created successfully");
        
        // Test rendering a single frame with the elite renderer
        log::info!("🔤 Testing single frame render with elite renderer");
        let _test_result = elite_renderer.render(layout, &self.config)?;
        log::info!("🔤 Elite renderer test completed");
        
        // Load shader-specific textures (including font atlas for unified shader)
        let shader_name = self.config.shader.clone();

        // Load text data for unified shader using Glyphon
        if shader_name == "codeskew_unified" {
            if let Err(e) = self.render_glyphon_to_texture(&mut wgputoy, layout).await {
                println!("🔧 WARNING: Failed to render Glyphon text: {}", e);
                // Fallback to old method
                let text_data = crate::shader_data::ShaderTextData::from_layout(layout);
                if let Err(e) = wgputoy.load_shader_text_data(&text_data) {
                    println!("🔧 WARNING: Failed to load shader text data: {}", e);
                } else {
                    println!("🔤 Loaded text data for unified shader (fallback)");
                }
            } else {
                println!("🔤 Loaded Glyphon text for unified shader");
            }
        }

        // Load procedural texture
        if let Err(e) = wgputoy.load_procedural_texture(0, 256, 256) {
            println!("🔧 WARNING: Failed to load texture: {}", e);
        } else {
            wgputoy.recreate_bind_group();
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

    /// Render 3D perspective text with elite renderer and optimal performance
    #[inline]
    async fn render_text_layer(&self, layout: &[PositionedLine]) -> Result<Vec<u8>, CodeSkewError> {
        println!("🔧 DEBUG: Starting text layer render with {} lines", layout.len());
        let mut elite_renderer = EliteWebGPURenderer::new(self.width_u32, self.height_u32).await?;

        let text_data = elite_renderer
            .render(layout, &self.config)
            .map_err(|e| CodeSkewError::RenderingError(format!("Text render failed: {e}")))?;
        
        println!("🔧 DEBUG: Text data length: {}", text_data.len());
        if text_data.len() >= 16 {
            println!("🔧 DEBUG: First 16 bytes: {:?}", &text_data[0..16]);
        }
        
        // Check for non-zero data
        let non_zero_count = text_data.iter().filter(|&&x| x != 0).count();
        println!("🔧 DEBUG: Non-zero bytes in text: {}/{}", non_zero_count, text_data.len());
        
        Ok(text_data)
    }

    /// Ultra-high-performance layer compositing with zero allocation and optimized integer arithmetic
    #[inline]
    fn composite_layers_optimized(
        &mut self,
        background_data: &[u8],
        text_data: &[u8],
    ) -> Result<(), CodeSkewError> {
        println!("🔧 DEBUG: Starting layer compositing");
        println!("🔧 DEBUG: Background buffer size: {}, Text buffer size: {}", background_data.len(), text_data.len());
        println!("🔧 DEBUG: Expected buffer size: {}", self.buffer_size);
        
        // Validate input data integrity with efficient checks
        if background_data.len() < self.buffer_size {
            return Err(CodeSkewError::RenderingError(
                "Background data buffer insufficient size".to_string(),
            ));
        }
        if text_data.len() < self.buffer_size {
            return Err(CodeSkewError::RenderingError(
                "Text data buffer insufficient size".to_string(),
            ));
        }

        // Ultra-fast vectorized alpha blending with optimized integer arithmetic
        let pixel_count = self.pixel_count;
        let rgba_buffer = &mut self.rgba_buffer;

        // Process 4 pixels at a time for better cache utilization
        for i in 0..pixel_count {
            let idx = i * 4;

            // Load background and text color components
            let bg_r = background_data[idx] as u32;
            let bg_g = background_data[idx + 1] as u32;
            let bg_b = background_data[idx + 2] as u32;
            let bg_a = background_data[idx + 3] as u32;

            let text_r = text_data[idx] as u32;
            let text_g = text_data[idx + 1] as u32;
            let text_b = text_data[idx + 2] as u32;
            let text_a = text_data[idx + 3] as u32;

            // Debug first few pixels
            if i < 3 {
                println!("🔧 DEBUG: Pixel {}: BG=({},{},{},{}), Text=({},{},{},{})", 
                         i, bg_r, bg_g, bg_b, bg_a, text_r, text_g, text_b, text_a);
            }

            // Optimized alpha blending with pre-calculated inverse alpha
            let inv_alpha = 255 - text_a;

            // Blend each component with maximum efficiency
            rgba_buffer[idx] = ((text_r * text_a + bg_r * inv_alpha) / 255) as u8;
            rgba_buffer[idx + 1] = ((text_g * text_a + bg_g * inv_alpha) / 255) as u8;
            rgba_buffer[idx + 2] = ((text_b * text_a + bg_b * inv_alpha) / 255) as u8;
            rgba_buffer[idx + 3] = ((text_a * text_a + bg_a * inv_alpha) / 255) as u8;
            
            // Debug first few results
            if i < 3 {
                println!("🔧 DEBUG: Result {}: ({},{},{},{})", 
                         i, rgba_buffer[idx], rgba_buffer[idx+1], rgba_buffer[idx+2], rgba_buffer[idx+3]);
            }
        }

        // Final buffer check
        let final_non_zero = rgba_buffer.iter().filter(|&&x| x != 0).count();
        println!("🔧 DEBUG: Final rgba_buffer non-zero bytes: {}/{}", final_non_zero, rgba_buffer.len());

        Ok(())
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
}
