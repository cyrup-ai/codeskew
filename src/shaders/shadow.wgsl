// Ultra-optimized shadow mapping shader for zero-allocation renderer

struct OptimizedVertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) normal: vec3<f32>,
    @location(4) depth_offset: f32,
    @location(5) material_id: u32,
    @location(6) light_space_pos: vec3<f32>,
    @location(7) screen_space_uv: vec2<f32>,
    @location(8) tangent: vec3<f32>,
    @location(9) bitangent: vec3<f32>,
}

struct UnifiedUniforms {
    model_view_proj: mat4x4<f32>,
    model_view: mat4x4<f32>,
    normal_matrix: mat4x4<f32>,
    light_view_proj: mat4x4<f32>,
    light_position: vec3<f32>,
    light_intensity: f32,
    light_direction: vec3<f32>,
    _light_pad1: f32,
    light_color: vec3<f32>,
    ambient_strength: f32,
    shadow_bias: f32,
    shadow_normal_bias: f32,
    shadow_radius: f32,
    _light_pad2: f32,
    camera_position: vec3<f32>,
    field_of_view: f32,
    camera_direction: vec3<f32>,
    near_plane: f32,
    far_plane: f32,
    exposure: f32,
    gamma: f32,
    _camera_pad: f32,
    viewport_size: vec2<f32>,
    inv_viewport_size: vec2<f32>,
    time: f32,
    delta_time: f32,
    frame_count: u32,
    _time_pad: u32,
}

@group(0) @binding(0)
var<uniform> uniforms: UnifiedUniforms;

@vertex
fn vs_main(vertex: OptimizedVertexInput) -> @builtin(position) vec4<f32> {
    // Shadow depth calculation - only position matters for shadow mapping
    let world_pos = vec4<f32>(vertex.position, 1.0);
    return uniforms.light_view_proj * world_pos;
}