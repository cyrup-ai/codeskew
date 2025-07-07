//! Composite renderer for combining multiple render layers

use crate::error::CodeSkewError;
use crate::toy::{WgpuContext, WgpuToyRenderer};
use crate::webgpu::EliteWebGPURenderer;
use std::sync::Arc;

const BACKGROUND_SHADER_TEMPLATE: &str = r#"
@group(0) @binding(0) var<storage, read_write> screen: array<vec4<f32>>;
@group(0) @binding(1) var<uniform> uniforms: Uniforms;

struct Uniforms {
    resolution: vec2<f32>,
    time: f32,
    _padding: f32,
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let pos = vec2<f32>(global_id.xy);
    let uv = pos / uniforms.resolution;

    let time = uniforms.time;
    let wave1 = sin(uv.x * 12.0 + time * 2.5) * 0.15;
    let wave2 = cos(uv.y * 8.0 + time * 1.8) * 0.1;

    let color1 = vec3<f32>(0.1 + wave1, 0.2 + wave2, 0.6 + sin(time * 0.5) * 0.2);
    let color2 = vec3<f32>(0.7 + cos(time * 0.7) * 0.2, 0.1 + wave2, 0.3 + wave1);

    let gradient = mix(color1, color2, smoothstep(0.0, 1.0, uv.y + sin(uv.x * 4.0 + time) * 0.1));

    let pixel_index = global_id.y * u32(uniforms.resolution.x) + global_id.x;
    screen[pixel_index] = vec4<f32>(gradient, 1.0);
}
"#;

const COMPOSITE_SHADER: &str = r#"
@group(0) @binding(0) var<storage, read> background_buffer: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read> text_buffer: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read_write> output_buffer: array<vec4<f32>>;
@group(0) @binding(3) var<uniform> uniforms: CompositeUniforms;

struct CompositeUniforms {
    resolution: vec2<f32>,
    _padding: vec2<f32>,
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let pixel_index = global_id.y * u32(uniforms.resolution.x) + global_id.x;

    if (pixel_index >= u32(uniforms.resolution.x * uniforms.resolution.y)) {
        return;
    }

    let background = background_buffer[pixel_index];
    let text = text_buffer[pixel_index];

    // Premultiplied alpha blending
    let inv_alpha = 1.0 - text.a;
    let result = vec4<f32>(
        text.rgb * text.a + background.rgb * inv_alpha,
        max(background.a, text.a)
    );

    output_buffer[pixel_index] = result;
}
"#;

pub struct CompositeRenderer {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub composite_texture: wgpu::Texture,
    pub copy_buffer: wgpu::Buffer,
    pub bytes_per_row: u32,
    pub width: u32,
    pub height: u32,
    animation_time: f32,
    toy_renderer: WgpuToyRenderer,
    elite_renderer: EliteWebGPURenderer,
    background_pipeline: wgpu::ComputePipeline,
    composite_pipeline: wgpu::ComputePipeline,
    background_buffer: wgpu::Buffer,
    text_buffer: wgpu::Buffer,
    output_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    composite_uniform_buffer: wgpu::Buffer,
    background_bind_group: wgpu::BindGroup,
    composite_bind_group: wgpu::BindGroup,
    pixel_count: u32,
    workgroup_count: (u32, u32),
    result_buffer: Vec<u8>,
}

impl CompositeRenderer {
    /// Create a new composite renderer with pre-allocated buffers
    #[allow(clippy::ptr_arg)] // Vec needed for resize() capability
    pub async fn new(
        wgpu_context: WgpuContext,
        width: u32,
        height: u32,
        rgba_buffer: &mut Vec<u8>,
        _temp_buffer: &mut Vec<u8>,
    ) -> Result<Self, CodeSkewError> {
        let device = wgpu_context.device;
        let queue = wgpu_context.queue;

        let pixel_count = width * height;
        let buffer_size = (pixel_count * 16) as u64; // vec4<f32> = 16 bytes
        let bytes_per_row = (width * 4 + 255) & !255;

        // Pre-allocate result buffer to avoid allocations in hot path
        rgba_buffer.resize((pixel_count * 4) as usize, 0);

        // Create GPU buffers
        let background_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Background Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let text_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Text Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Uniform Buffer"),
            size: 16, // vec2<f32> + f32 + padding
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let composite_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Composite Uniform Buffer"),
            size: 16, // vec2<f32> + padding
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create composite texture
        let composite_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Composite Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let copy_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Copy Buffer"),
            size: (bytes_per_row * height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // Create compute pipelines
        let background_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Background Shader"),
            source: wgpu::ShaderSource::Wgsl(BACKGROUND_SHADER_TEMPLATE.into()),
        });

        let composite_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Composite Shader"),
            source: wgpu::ShaderSource::Wgsl(COMPOSITE_SHADER.into()),
        });

        let background_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Background Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let composite_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Composite Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let background_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Background Pipeline Layout"),
                bind_group_layouts: &[&background_bind_group_layout],
                push_constant_ranges: &[],
            });

        let composite_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Composite Pipeline Layout"),
                bind_group_layouts: &[&composite_bind_group_layout],
                push_constant_ranges: &[],
            });

        let background_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Background Pipeline"),
                layout: Some(&background_pipeline_layout),
                module: &background_shader,
                entry_point: Some("main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

        let composite_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Composite Pipeline"),
            layout: Some(&composite_pipeline_layout),
            module: &composite_shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        // Create bind groups
        let background_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Background Bind Group"),
            layout: &background_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: background_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

        let composite_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Composite Bind Group"),
            layout: &composite_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: background_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: text_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: composite_uniform_buffer.as_entire_binding(),
                },
            ],
        });

        // Initialize renderers
        let toy_renderer = WgpuToyRenderer::new(WgpuContext {
            device: device.clone(),
            queue: queue.clone(),
            surface: wgpu_context.surface,
            surface_config: wgpu_context.surface_config,
            #[cfg(all(not(target_arch = "wasm32"), feature = "winit"))]
            event_loop: wgpu_context.event_loop,
            #[cfg(all(not(target_arch = "wasm32"), feature = "winit"))]
            window: wgpu_context.window,
        });

        let elite_renderer = EliteWebGPURenderer::new(width, height).await?;

        let workgroup_count = (width.div_ceil(16), height.div_ceil(16));

        Ok(Self {
            device,
            queue: Arc::new(queue),
            composite_texture,
            copy_buffer,
            bytes_per_row,
            width,
            height,
            animation_time: 0.0,
            toy_renderer,
            elite_renderer,
            background_pipeline,
            composite_pipeline,
            background_buffer,
            text_buffer,
            output_buffer,
            uniform_buffer,
            composite_uniform_buffer,
            background_bind_group,
            composite_bind_group,
            pixel_count,
            workgroup_count,
            result_buffer: Vec::with_capacity((pixel_count * 4) as usize),
        })
    }

    /// Render composite frame with background and text layers
    pub async fn render_frame(
        &mut self,
        layout: &[crate::layout::PositionedLine],
    ) -> Result<Vec<u8>, CodeSkewError> {
        self.animation_time += 0.016;

        // Update uniforms
        let uniforms = [
            self.width as f32,
            self.height as f32,
            self.animation_time,
            0.0, // padding
        ];

        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&uniforms));

        let composite_uniforms = [
            self.width as f32,
            self.height as f32,
            0.0, // padding
            0.0, // padding
        ];

        self.queue.write_buffer(
            &self.composite_uniform_buffer,
            0,
            bytemuck::cast_slice(&composite_uniforms),
        );

        // Create command encoder
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Composite Render"),
            });

        // Step 1: Generate background using toy renderer and compute shader
        {
            // Use the toy_renderer to generate the background data
            let background_data = self.toy_renderer.render_to_buffer().await
                .map_err(|e| CodeSkewError::RenderingError(format!("Toy renderer failed: {e}")))?;
            
            // Convert background data to f32 format and upload to background_buffer
            let background_f32_data: Vec<f32> = background_data
                .chunks_exact(4)
                .flat_map(|chunk| {
                    [
                        chunk[0] as f32 / 255.0,
                        chunk[1] as f32 / 255.0,
                        chunk[2] as f32 / 255.0,
                        chunk[3] as f32 / 255.0,
                    ]
                })
                .collect();

            self.queue.write_buffer(&self.background_buffer, 0, bytemuck::cast_slice(&background_f32_data));

            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Background Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.background_pipeline);
            compute_pass.set_bind_group(0, &self.background_bind_group, &[]);
            compute_pass.dispatch_workgroups(self.workgroup_count.0, self.workgroup_count.1, 1);
        }

        // Step 2: Render text using elite renderer
        let text_config = crate::config::Config {
            width: self.width,
            height: self.height,
            input: std::path::PathBuf::new(),
            output: std::path::PathBuf::new(),
            font: "monospace".to_string(),
            fontsize: 12.0,
            skew: 0.0,
            depth: 0.0,
            perspective: 0.0,
            blur: 0.0,
            animate: false,
            theme: "dark".to_string(),
            centered: false,
            gradient: crate::config::GradientColors::new(
                "#000000".to_string(),
                "#ffffff".to_string(),
            ),
            format: crate::cli::OutputFormat::Png,
            telegram: false,
            duration: 1.0,
            fps: 30.0,
            shader: "zeroshot".to_string(),

            // Ligature configuration - use default
            ligature_config: crate::glyphon::ligature_config::LigatureConfig::default(),
            
            // 3D perspective parameters
            fold: 0.4,
            skew_angle: 0.15,
            scale: 0.6,
        };

        let text_data = self.elite_renderer.render(layout, &text_config)?;

        // Convert text data to f32 and upload to GPU
        let text_f32_data: Vec<f32> = text_data
            .chunks_exact(4)
            .flat_map(|chunk| {
                [
                    chunk[0] as f32 / 255.0,
                    chunk[1] as f32 / 255.0,
                    chunk[2] as f32 / 255.0,
                    chunk[3] as f32 / 255.0,
                ]
            })
            .collect();

        self.queue
            .write_buffer(&self.text_buffer, 0, bytemuck::cast_slice(&text_f32_data));

        // Step 3: Composite layers using compute shader
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Composite Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.composite_pipeline);
            compute_pass.set_bind_group(0, &self.composite_bind_group, &[]);
            compute_pass.dispatch_workgroups(self.workgroup_count.0, self.workgroup_count.1, 1);
        }

        // Step 4: Copy result to readback buffer
        encoder.copy_buffer_to_buffer(
            &self.output_buffer,
            0,
            &self.copy_buffer,
            0,
            (self.pixel_count * 16) as u64,
        );

        // Submit commands
        self.queue.submit(std::iter::once(encoder.finish()));

        // Map and read result
        let buffer_slice = self.copy_buffer.slice(..);
        let (sender, receiver) = flume::unbounded();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });

        let _ = self.device.poll(wgpu::MaintainBase::Wait);

        receiver
            .recv_async()
            .await
            .map_err(|_| {
                CodeSkewError::RenderingError("Failed to receive buffer map result".to_string())
            })?
            .map_err(|e| {
                CodeSkewError::RenderingError(format!("Buffer mapping failed: {e:?}"))
            })?;

        let data = buffer_slice.get_mapped_range();
        let f32_data: &[f32] = bytemuck::cast_slice(&data);

        // Convert f32 data back to u8 and reuse pre-allocated buffer
        self.result_buffer.clear();
        self.result_buffer
            .extend(f32_data.chunks_exact(4).flat_map(|chunk| {
                [
                    (chunk[0] * 255.0) as u8,
                    (chunk[1] * 255.0) as u8,
                    (chunk[2] * 255.0) as u8,
                    (chunk[3] * 255.0) as u8,
                ]
            }));

        drop(data);
        self.copy_buffer.unmap();

        Ok(self.result_buffer.clone())
    }

    /// Update animation time
    #[inline]
    pub fn update_animation(&mut self, delta_time: f32) {
        self.animation_time += delta_time;
    }

    /// Get current animation time
    #[inline]
    pub fn animation_time(&self) -> f32 {
        self.animation_time
    }

    /// Get render dimensions
    #[inline]
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}
