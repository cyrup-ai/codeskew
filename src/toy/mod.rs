pub mod bind;
pub mod blit;
pub mod context;
pub mod pp;
pub mod utils;

pub use bind::*;
pub use blit::*;
pub use context::{WgpuContext, init_wgpu};
pub use pp::{SourceMap, WGSLError};
pub use utils::*;

use lazy_regex::regex;
use wasm_bindgen::prelude::*;

struct ComputePipeline {
    _name: String,
    pipeline: wgpu::ComputePipeline,
    workgroup_size: [u32; 3],
    workgroup_count: Option<[u32; 3]>,
    _dispatch_once: bool,
    _dispatch_count: u32,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone)]
struct SuccessCallback(Option<js_sys::Function>);

#[cfg(not(target_arch = "wasm32"))]
struct SuccessCallback(Option<()>);

#[wasm_bindgen]
pub struct WgpuToyRenderer {
    #[wasm_bindgen(skip)]
    pub wgpu: WgpuContext,
    screen_width: u32,
    screen_height: u32,
    #[wasm_bindgen(skip)]
    pub bindings: bind::Bindings,
    compute_pipeline_layout: wgpu::PipelineLayout,
    _last_compute_pipelines: Option<Vec<ComputePipeline>>,
    compute_pipelines: Vec<ComputePipeline>,
    compute_bind_group: wgpu::BindGroup,
    compute_bind_group_layout: wgpu::BindGroupLayout,
    _on_success_cb: SuccessCallback,
    _pass_f32: bool,
    _screen_blitter: blit::Blitter,
    _query_set: Option<wgpu::QuerySet>,
    _last_stats: instant::Instant,
    source: SourceMap,
}

impl WgpuToyRenderer {
    pub fn new(wgpu: WgpuContext) -> WgpuToyRenderer {
        let bindings = bind::Bindings::new(
            &wgpu,
            wgpu.surface_config.width,
            wgpu.surface_config.height,
            false,
        );
        let layout = bindings.create_bind_group_layout(&wgpu);

        WgpuToyRenderer {
            compute_pipeline_layout: bindings.create_pipeline_layout(&wgpu, &layout),
            compute_bind_group: bindings.create_bind_group(&wgpu, &layout),
            compute_bind_group_layout: layout,
            _last_compute_pipelines: None,
            compute_pipelines: vec![],
            screen_width: wgpu.surface_config.width,
            screen_height: wgpu.surface_config.height,
            _screen_blitter: blit::Blitter::new(
                &wgpu,
                bindings.tex_screen.view(),
                blit::ColourSpace::Linear,
                wgpu.surface_config.format,
                wgpu::FilterMode::Nearest,
            ),
            wgpu,
            bindings,
            _on_success_cb: SuccessCallback(None),
            _pass_f32: false,
            _query_set: None,
            _last_stats: instant::Instant::now(),
            source: SourceMap::new(),
        }
    }

    /// Compile shader source into compute pipeline - the missing piece!
    pub fn compile(&mut self, source: pp::SourceMap) {
        let now = instant::Instant::now();

        // Generate prelude with binding declarations
        let prelude = self.prelude();
        let wgsl = format!("{}{}", prelude, source.source);

        // Parse entry points from the complete WGSL
        let re_entry_point = regex!(r"(?s)@compute.*?@workgroup_size\((.*?)\).*?fn\s+(\w+)");
        let entry_points: Vec<(String, [u32; 3])> = re_entry_point
            .captures_iter(&wgsl)
            .map(|cap| {
                let mut sizes = cap[1].split(',').map(|s| s.trim().parse().unwrap_or(1));
                let workgroup_size: [u32; 3] = std::array::from_fn(|_| sizes.next().unwrap_or(1));
                (cap[2].to_owned(), workgroup_size)
            })
            .collect();

        // Create compute pipelines from entry points
        self.compute_pipelines.clear();
        for (entry_point, workgroup_size) in entry_points {
            let compute_shader = self.wgpu.device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(&format!("Compute Shader - {}", entry_point)),
                source: wgpu::ShaderSource::Wgsl(wgsl.clone().into()),
            });

            let compute_pipeline = self.wgpu.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(&format!("Compute Pipeline - {}", entry_point)),
                layout: Some(&self.compute_pipeline_layout),
                module: &compute_shader,
                entry_point: Some(&entry_point),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

            self.compute_pipelines.push(ComputePipeline {
                _name: entry_point.clone(),
                pipeline: compute_pipeline,
                workgroup_size,
                workgroup_count: source.workgroup_count.get(&entry_point).cloned(),
                _dispatch_once: *source.dispatch_once.get(&entry_point).unwrap_or(&false),
                _dispatch_count: *source.dispatch_count.get(&entry_point).unwrap_or(&1),
            });
        }

        println!("🔧 DEBUG: Compiled {} compute pipelines in {:.3}s",
                 self.compute_pipelines.len(),
                 now.elapsed().as_secs_f32());

        self.source = source;
    }

    /// Generate prelude with all binding declarations
    fn prelude(&self) -> String {
        let mut s = String::new();

        // Type aliases for convenience
        for (a, t) in [("int", "i32"), ("uint", "u32"), ("float", "f32")] {
            s.push_str(&format!("alias {a} = {t};\n"));
        }
        for (a, t) in [("int", "i32"), ("uint", "u32"), ("float", "f32"), ("bool", "bool")] {
            for n in 2..5 {
                s.push_str(&format!("alias {a}{n} = vec{n}<{t}>;\n"));
            }
        }
        for n in 2..5 {
            for m in 2..5 {
                s.push_str(&format!("alias float{n}x{m} = mat{n}x{m}<f32>;\n"));
            }
        }

        // Standard structs
        s.push_str(r#"
struct Time { frame: uint, elapsed: float, delta: float }
struct Mouse { pos: uint2, click: int }
struct DispatchInfo { id: uint }
"#);

        // Custom struct (dynamic generation)
        s.push_str("struct Custom {\n");
        let (custom_names, _) = &self.bindings.custom.host;
        for name in custom_names {
            s.push_str(&format!("    {}: float,\n", name));
        }
        s.push_str("};\n");

        // Data struct (dynamic generation)
        s.push_str("struct Data {\n");
        for (key, val) in self.bindings.user_data.host.iter() {
            let n = val.len();
            s.push_str(&format!("    {}: array<u32,{}>,\n", key, n));
        }
        s.push_str("};\n");

        // All binding declarations
        s.push_str(&self.bindings.to_wgsl());

        // Helper functions
        s.push_str(r#"
fn keyDown(keycode: uint) -> bool {
    return ((_keyboard[keycode / 128u][(keycode % 128u) / 32u] >> (keycode % 32u)) & 1u) == 1u;
}

fn assert(index: int, success: bool) {
    if (!success) {
        atomicAdd(&_assert_counts[index], 1u);
    }
}

fn passStore(pass_index: int, coord: int2, value: float4) {
    textureStore(pass_out, coord, pass_index, value);
}

fn passLoad(pass_index: int, coord: int2, lod: int) -> float4 {
    return textureLoad(pass_in, coord, pass_index, lod);
}

fn passSampleLevelBilinearRepeat(pass_index: int, uv: float2, lod: float) -> float4 {
    return textureSampleLevel(pass_in, bilinear_repeat, fract(uv), pass_index, lod);
}
"#);

        s
    }

    pub async fn preprocess_async(&mut self, shader: &str) -> Option<SourceMap> {
        let defines = rustc_hash::FxHashMap::from_iter([
            ("SCREEN_WIDTH".to_owned(), self.screen_width.to_string()),
            ("SCREEN_HEIGHT".to_owned(), self.screen_height.to_string()),
        ]);
        pp::Preprocessor::new(defines).run(shader).await
    }
}

// Add the render_to_buffer method and other essential functionality
impl WgpuToyRenderer {
    pub async fn render_to_buffer(&mut self) -> Result<Vec<u8>, String> {
        // Update time for animation
        self.bindings.time.host.elapsed += 0.016; // ~60fps
        self.bindings.time.host.frame = self.bindings.time.host.frame.wrapping_add(1);

        // Stage uniform data
        self.bindings.stage(&self.wgpu.queue);

        // Create command encoder
        let mut encoder = self.wgpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        // Dispatch compute shaders
        for pipeline in &self.compute_pipelines {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Pass"),
                timestamp_writes: None,
            });

            let workgroup_count = pipeline.workgroup_count.unwrap_or([
                self.screen_width.div_ceil(pipeline.workgroup_size[0]),
                self.screen_height.div_ceil(pipeline.workgroup_size[1]),
                1,
            ]);

            compute_pass.set_pipeline(&pipeline.pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[0]);
            compute_pass.dispatch_workgroups(
                workgroup_count[0],
                workgroup_count[1],
                workgroup_count[2],
            );
        }

        // Create staging buffer for readback (rgba16float = 8 bytes per pixel)
        let bytes_per_row = (self.screen_width * 8).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let staging_buffer = self.wgpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging Buffer"),
            size: (bytes_per_row * self.screen_height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // Copy texture to staging buffer
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: self.bindings.tex_screen.texture(),
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &staging_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(self.screen_height),
                },
            },
            wgpu::Extent3d {
                width: self.screen_width,
                height: self.screen_height,
                depth_or_array_layers: 1,
            },
        );

        // Submit commands
        let _submission_index = self.wgpu.queue.submit(std::iter::once(encoder.finish()));

        // Map buffer for reading
        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = futures::channel::oneshot::channel();

        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });

        // Wait for GPU operations to complete
        self.wgpu.device.poll(wgpu::MaintainBase::Wait);

        // Receive mapping result
        match receiver.await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                return Err(format!("Buffer mapping failed: {e:?}"));
            }
            Err(_) => {
                return Err("Channel error during buffer mapping".to_string());
            }
        }

        // Read the data and convert from f16 to u8
        let data = buffer_slice.get_mapped_range();
        let f16_data: &[u8] = &data;

        // Convert from rgba16float to rgba8unorm
        let mut result = Vec::with_capacity((self.screen_width * self.screen_height * 4) as usize);

        // Process row by row to handle padding
        for y in 0..self.screen_height {
            let row_offset = (y * bytes_per_row) as usize;
            for x in 0..self.screen_width {
                let pixel_offset = row_offset + (x * 8) as usize;
                if pixel_offset + 8 <= f16_data.len() {
                    let chunk = &f16_data[pixel_offset..pixel_offset + 8];
                    // Convert f16 to f32 then to u8
                    let r = half::f16::from_le_bytes([chunk[0], chunk[1]]).to_f32();
                    let g = half::f16::from_le_bytes([chunk[2], chunk[3]]).to_f32();
                    let b = half::f16::from_le_bytes([chunk[4], chunk[5]]).to_f32();
                    let a = half::f16::from_le_bytes([chunk[6], chunk[7]]).to_f32();

                    result.push((r.clamp(0.0, 1.0) * 255.0) as u8);
                    result.push((g.clamp(0.0, 1.0) * 255.0) as u8);
                    result.push((b.clamp(0.0, 1.0) * 255.0) as u8);
                    result.push((a.clamp(0.0, 1.0) * 255.0) as u8);
                }
            }
        }

        drop(data);
        staging_buffer.unmap();

        Ok(result)
    }
}

// Add essential async and sync rendering methods
impl WgpuToyRenderer {
    pub async fn create_renderer(
        width: u32,
        height: u32,
        bind_id: String,
    ) -> Result<WgpuToyRenderer, String> {
        let wgpu = init_wgpu(width, height, &bind_id).await?;
        Ok(WgpuToyRenderer::new(wgpu))
    }

    /// Load texture data into a channel
    pub fn load_channel(&mut self, channel: usize, data: &[u8]) -> Result<(), String> {
        if channel >= self.bindings.channels.len() {
            return Err(format!("Channel {} does not exist", channel));
        }

        let image = image::load_from_memory(data)
            .map_err(|e| format!("Failed to load image: {}", e))?;
        let rgba_image = image.to_rgba8();

        let texture_desc = wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: rgba_image.width(),
                height: rgba_image.height(),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        };

        let texture = self.wgpu.device.create_texture(&texture_desc);

        self.wgpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba_image,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * rgba_image.width()),
                rows_per_image: Some(rgba_image.height()),
            },
            texture_desc.size,
        );

        self.bindings.channels[channel].set_texture(texture);
        Ok(())
    }

    /// Generate a simple procedural texture for testing
    pub fn load_procedural_texture(&mut self, channel: usize, width: u32, height: u32) -> Result<(), String> {
        if channel >= self.bindings.channels.len() {
            return Err(format!("Channel {} does not exist", channel));
        }

        // Generate noise texture data for zeroshot shader
        let mut data = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                // Create pseudo-random noise pattern
                let noise = ((x.wrapping_mul(374761393).wrapping_add(y.wrapping_mul(668265263))) ^ (x * y)) % 256;
                let noise_f = noise as f32 / 255.0;

                // Convert to grayscale noise with some variation
                let val = (noise_f * 255.0) as u8;
                data.push(val); // R
                data.push(val); // G
                data.push(val); // B
                data.push(255); // A
            }
        }

        let texture_desc = wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        };

        let texture = self.wgpu.device.create_texture(&texture_desc);

        self.wgpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            texture_desc.size,
        );

        self.bindings.channels[channel].set_texture(texture);
        Ok(())
    }

    /// Recreate bind group after updating textures
    pub fn recreate_bind_group(&mut self) {
        self.compute_bind_group = self.bindings.create_bind_group(&self.wgpu, &self.compute_bind_group_layout);
    }

    /// Set a custom texture for a specific channel
    pub fn set_channel_texture(&mut self, channel: usize, texture: wgpu::Texture) -> Result<(), String> {
        if channel >= self.bindings.channels.len() {
            return Err(format!("Channel {} does not exist", channel));
        }

        self.bindings.channels[channel].set_texture(texture);
        self.recreate_bind_group();
        Ok(())
    }

    /// Load shader text data (ASCII grid + colors) for unified compute shader
    pub fn load_shader_text_data(&mut self, text_data: &crate::shader_data::ShaderTextData) -> Result<(), String> {
        // Upload terminal grid (ASCII characters) to storage1
        let terminal_buffer = text_data.to_terminal_buffer();
        self.wgpu.queue.write_buffer(
            self.bindings.storage1.buffer(),
            0,
            bytemuck::cast_slice(&terminal_buffer)
        );

        // Upload color grid (syntax highlighting colors) to storage2
        let color_buffer = text_data.to_color_buffer();
        self.wgpu.queue.write_buffer(
            self.bindings.storage2.buffer(),
            0,
            bytemuck::cast_slice(&color_buffer)
        );

        println!("🔤 Uploaded shader text data: {} characters, {} colors",
                 terminal_buffer.len(), color_buffer.len());

        Ok(())
    }

    /// Reset time and buffers - copied from wgpu-compute-toy
    pub fn reset(&mut self) {
        let mut bindings = bind::Bindings::new(
            &self.wgpu,
            self.screen_width,
            self.screen_height,
            false,
        );
        std::mem::swap(&mut self.bindings, &mut bindings);
        self.bindings.custom.host = bindings.custom.host.clone();
        self.bindings.user_data.host = bindings.user_data.host.clone();
        // self.bindings.channels = std::mem::take(&mut bindings.channels);
        let layout = self.bindings.create_bind_group_layout(&self.wgpu);
        self.compute_pipeline_layout = self.bindings.create_pipeline_layout(&self.wgpu, &layout);
        self.compute_bind_group = self.bindings.create_bind_group(&self.wgpu, &layout);
        self.compute_bind_group_layout = layout;
    }

    /// Set time elapsed - copied from wgpu-compute-toy
    pub fn set_time_elapsed(&mut self, t: f32) {
        self.bindings.time.host.elapsed = t;
    }

    /// Set mouse position - copied from wgpu-compute-toy
    pub fn set_mouse_pos(&mut self, x: f32, y: f32) {
        if self.bindings.mouse.host.click == 1 {
            self.bindings.mouse.host.pos = [
                (x * self.screen_width as f32) as u32,
                (y * self.screen_height as f32) as u32,
            ];
        }
    }

    /// Set mouse click state - copied from wgpu-compute-toy
    pub fn set_mouse_click(&mut self, click: bool) {
        self.bindings.mouse.host.click = if click { 1 } else { 0 };
    }

    /// Resize renderer - copied from wgpu-compute-toy
    pub fn resize(&mut self, width: u32, height: u32, scale: f32) {
        self.screen_width = (width as f32 * scale) as u32;
        self.screen_height = (height as f32 * scale) as u32;
        self.wgpu.surface_config.width = self.screen_width;
        self.wgpu.surface_config.height = self.screen_height;
        self.wgpu
            .surface
            .configure(&self.wgpu.device, &self.wgpu.surface_config);
        self.reset();
    }

    /// Async render method - copied from wgpu-compute-toy
    pub async fn render_async(&mut self) {
        use wgpu::SurfaceError;

        match self.wgpu.surface.get_current_texture() {
            Err(err) => match err {
                SurfaceError::Lost | SurfaceError::Outdated => {
                    log::error!("Unable to get framebuffer: {err}");
                    self.wgpu.surface.configure(&self.wgpu.device, &self.wgpu.surface_config);
                    #[cfg(feature = "winit")]
                    self.wgpu.window.request_redraw();
                }
                SurfaceError::OutOfMemory => log::error!("Out of GPU Memory!"),
                SurfaceError::Timeout => log::warn!("Surface Timeout"),
                SurfaceError::Other => log::error!("Other surface error: {err}"),
            },
            Ok(frame) => {
                self.render_to_surface(&frame);
                frame.present();
            }
        }
    }

    /// Render to surface - simplified version from wgpu-compute-toy
    fn render_to_surface(&mut self, frame: &wgpu::SurfaceTexture) {
        let mut encoder = self.wgpu.device.create_command_encoder(&Default::default());

        // Stage uniform data
        self.bindings.stage(&self.wgpu.queue);

        // Dispatch compute shaders
        for pipeline in &self.compute_pipelines {
            let mut compute_pass = encoder.begin_compute_pass(&Default::default());

            let workgroup_count = pipeline.workgroup_count.unwrap_or([
                self.screen_width.div_ceil(pipeline.workgroup_size[0]),
                self.screen_height.div_ceil(pipeline.workgroup_size[1]),
                1,
            ]);

            compute_pass.set_pipeline(&pipeline.pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[0]);
            compute_pass.dispatch_workgroups(
                workgroup_count[0],
                workgroup_count[1],
                workgroup_count[2],
            );
        }

        // Use the blitter to copy from compute texture to surface
        // Recreate blitter if texture view is invalid (surgical fix for texture destruction)
        self._screen_blitter = blit::Blitter::new(
            &self.wgpu,
            self.bindings.tex_screen.view(),
            blit::ColourSpace::Linear,
            self.wgpu.surface_config.format,
            wgpu::FilterMode::Nearest,
        );
        self._screen_blitter.blit(&mut encoder, &frame.texture.create_view(&Default::default()));

        // Submit commands
        self.wgpu.queue.submit(std::iter::once(encoder.finish()));

        // Update frame counter
        self.bindings.time.host.frame = self.bindings.time.host.frame.wrapping_add(1);
    }
}
