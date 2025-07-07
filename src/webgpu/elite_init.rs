//! Elite WebGPU renderer structure and initialization

use crate::error::CodeSkewError;
use crate::webgpu::{CommandBufferState, ShaderUniforms, TextRenderState};
use glyphon::*;
use std::collections::HashMap;
use std::sync::Arc;

/// High-performance WebGPU renderer with animated WGSL shader backgrounds and text
pub struct EliteWebGPURenderer {
    // Core GPU resources
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,

    // Render dimensions
    pub width: u32,
    pub height: u32,

    // Shader animation state
    pub shader_pipeline: Option<wgpu::RenderPipeline>,
    pub compute_pipeline: Option<wgpu::ComputePipeline>,
    pub shader_bind_group: Option<wgpu::BindGroup>,
    pub uniform_buffer: wgpu::Buffer,
    pub uniforms: ShaderUniforms,
    pub texture_bind_groups: HashMap<usize, wgpu::BindGroup>,
    #[allow(dead_code)] // Public API - texture loading system for external use
    pub loaded_textures: HashMap<usize, wgpu::Texture>,
    #[allow(dead_code)] // Public API - custom shader parameters for external use
    pub custom_floats: HashMap<String, f32>,
    #[allow(dead_code)] // Public API - keyboard input state for external use
    pub keyboard_state: [bool; 256],
    pub shader_loaded: bool,
    pub frame_count: f32,
    pub time_elapsed: f32,
    pub time_delta: f32,
    pub last_frame_time: std::time::Instant,

    // Text rendering resources
    pub font_system: FontSystem,
    pub cache: SwashCache,
    #[allow(dead_code)] // Public API - text rendering cache for external use
    pub glyphon_cache: Cache,
    pub viewport: Viewport,
    pub text_atlas: TextAtlas,
    pub text_renderer: TextRenderer,

    // Render targets
    #[allow(dead_code)] // Public API - background texture for external use
    pub background_texture: wgpu::Texture,
    pub background_view: wgpu::TextureView,
    pub composite_texture: wgpu::Texture,
    pub composite_view: wgpu::TextureView,
    
    // Supersampling configuration
    pub supersampling_factor: f32,
    
    // Supersampled render targets
    pub supersampled_texture: wgpu::Texture,
    pub supersampled_view: wgpu::TextureView,
    pub supersampled_width: u32,
    pub supersampled_height: u32,
    
    // Downsampling resources
    pub downsample_pipeline: wgpu::RenderPipeline,
    pub downsample_bind_group: wgpu::BindGroup,
    pub downsample_bind_group_layout: wgpu::BindGroupLayout,
    pub downsample_uniform_buffer: wgpu::Buffer,

    // Buffer management
    pub copy_buffer: wgpu::Buffer,

    // Memory layout parameters
    pub padded_bytes_per_row: u32,
    pub unpadded_bytes_per_row: u32,

    // Text rendering state
    pub text_state: TextRenderState,

    // Command buffer management
    pub command_state: CommandBufferState,

    // Result buffer
    pub result_data: Vec<u8>,

    // String building buffer for reuse
    pub string_builder: String,

    // Composite pass resources
    pub composite_pipeline: wgpu::RenderPipeline,
    #[allow(dead_code)] // Public API - composite rendering pipeline for external use
    pub composite_bind_group_layout: wgpu::BindGroupLayout,
    #[allow(dead_code)] // Public API - composite rendering pipeline for external use
    pub composite_bind_group: wgpu::BindGroup,
    #[allow(dead_code)] // Public API - texture sampling for external use
    pub sampler: wgpu::Sampler,

    // Shader bind group layouts
    #[allow(dead_code)] // Public API - shader binding system for external use
    pub shader_bind_group_layout: Option<wgpu::BindGroupLayout>,
    #[allow(dead_code)] // Public API - texture binding system for external use
    pub texture_bind_group_layout: Option<wgpu::BindGroupLayout>,
}

impl EliteWebGPURenderer {
    /// Creates a new renderer with animated WGSL background support
    pub async fn new(width: u32, height: u32) -> Result<Self, CodeSkewError> {
        // Use a default base font size - this will be overridden by actual calculated size
        Self::new_with_supersampling(width, height, 3.0, 16.0).await
    }
    
    /// Creates a new renderer with configurable supersampling factor and base font size
    pub async fn new_with_supersampling(width: u32, height: u32, supersampling_factor: f32, base_font_size: f32) -> Result<Self, CodeSkewError> {
        // Validate dimensions
        if width == 0 || height == 0 {
            return Err(CodeSkewError::RenderingError(format!(
                "Invalid dimensions: {width}x{height} - width and height must be non-zero"
            )));
        }

        // Initialize GPU instance with high performance preference
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            flags: wgpu::InstanceFlags::from_build_config(),
            backend_options: Default::default(),
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .map_err(|e| {
                CodeSkewError::RenderingError(format!("Failed to get WebGPU adapter: {e:?}"))
            })?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Elite WebGPU Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(|e| {
                CodeSkewError::RenderingError(format!("Failed to create device: {e}"))
            })?;

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        // Create uniform buffer
        let uniforms = ShaderUniforms::new();
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Uniform Buffer"),
            size: std::mem::size_of::<ShaderUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Initialize text rendering system
        let mut font_system = FontSystem::new();
        let cache = SwashCache::new();
        let glyphon_cache = Cache::new(&device);
        let mut viewport = Viewport::new(&device, &glyphon_cache);
        let mut text_atlas = TextAtlas::new(
            &device,
            &queue,
            &glyphon_cache,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        );
        let text_renderer = TextRenderer::new(
            &mut text_atlas,
            &device,
            wgpu::MultisampleState::default(),
            None,
        );

        // Calculate supersampled dimensions
        let supersampled_width = (width as f32 * supersampling_factor) as u32;
        let supersampled_height = (height as f32 * supersampling_factor) as u32;
        
        // Update viewport for supersampled rendering
        viewport.update(&queue, Resolution { width: supersampled_width, height: supersampled_height });
        
        // Create render targets
        let background_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Background Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

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
        
        // Create supersampled texture (3x resolution)
        let supersampled_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Supersampled Texture"),
            size: wgpu::Extent3d {
                width: supersampled_width,
                height: supersampled_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let background_view =
            background_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let composite_view = composite_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let supersampled_view = supersampled_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create copy buffer for readback
        let bytes_per_row = (width * 4 + 255) & !255;
        let copy_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Copy Buffer"),
            size: (bytes_per_row * height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // Create sampler
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Texture Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Create composite pipeline (placeholder)
        let composite_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Composite Bind Group Layout"),
                entries: &[],
            });

        let composite_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Composite Bind Group"),
            layout: &composite_bind_group_layout,
            entries: &[],
        });

        let composite_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Composite Pipeline Layout"),
                bind_group_layouts: &[&composite_bind_group_layout],
                push_constant_ranges: &[],
            });

        // Simple composite shader
        let composite_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Composite Shader"),
            source: wgpu::ShaderSource::Wgsl(r#"
                @vertex
                fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
                    let x = f32((vertex_index & 1u) << 1u) - 1.0;
                    let y = f32((vertex_index & 2u)) - 1.0;
                    return vec4<f32>(x, y, 0.0, 1.0);
                }

                @fragment
                fn fs_main() -> @location(0) vec4<f32> {
                    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
                }
            "#.into()),
        });

        let composite_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Composite Pipeline"),
            layout: Some(&composite_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &composite_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &composite_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        
        // Create downsampling pipeline for high-quality 3x to 1x reduction
        let downsample_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Downsample Bind Group Layout"),
            entries: &[
                // Input texture (supersampled)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                // Sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // Uniforms
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        
        // Create downsample uniform buffer
        let downsample_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Downsample Uniform Buffer"),
            size: 16, // f32 + 3*f32 padding
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        // Write the supersampling factor to the uniform buffer
        queue.write_buffer(
            &downsample_uniform_buffer,
            0,
            bytemuck::cast_slice(&[supersampling_factor, 0.0f32, 0.0f32, 0.0f32]),
        );
        
        let downsample_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Downsample Bind Group"),
            layout: &downsample_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&supersampled_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &downsample_uniform_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });
        
        let downsample_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Downsample Pipeline Layout"),
            bind_group_layouts: &[&downsample_bind_group_layout],
            push_constant_ranges: &[],
        });
        
        // High-quality downsampling shader with Lanczos filter
        let downsample_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Downsample Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/downsample.wgsl").into()),
        });
        
        let downsample_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Downsample Pipeline"),
            layout: Some(&downsample_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &downsample_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &downsample_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Use default font size for buffer creation - will be updated with actual size during rendering
        let text_state = TextRenderState::new(64, &mut font_system, supersampled_width, supersampled_height, supersampling_factor, 14.0);
        let command_state = CommandBufferState::new("Elite Renderer");

        Ok(Self {
            device,
            queue,
            width,
            height,
            shader_pipeline: None,
            compute_pipeline: None,
            shader_bind_group: None,
            uniform_buffer,
            uniforms,
            texture_bind_groups: HashMap::new(),
            loaded_textures: HashMap::new(),
            custom_floats: HashMap::new(),
            keyboard_state: [false; 256],
            shader_loaded: false,
            frame_count: 0.0,
            time_elapsed: 0.0,
            time_delta: 0.016,
            last_frame_time: std::time::Instant::now(),
            font_system,
            cache,
            glyphon_cache,
            viewport,
            text_atlas,
            text_renderer,
            background_texture,
            background_view,
            composite_texture,
            composite_view,
            copy_buffer,
            padded_bytes_per_row: bytes_per_row,
            unpadded_bytes_per_row: width * 4,
            text_state,
            command_state,
            result_data: vec![0u8; (width * height * 4) as usize],
            string_builder: String::new(),
            composite_pipeline,
            composite_bind_group_layout,
            composite_bind_group,
            sampler,
            shader_bind_group_layout: None,
            texture_bind_group_layout: None,
            supersampled_texture,
            supersampled_view,
            supersampled_width,
            supersampled_height,
            supersampling_factor,
            downsample_pipeline,
            downsample_bind_group,
            downsample_bind_group_layout,
            downsample_uniform_buffer,
        })
    }
}
