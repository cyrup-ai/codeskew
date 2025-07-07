// Vertex shader
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) world_position: vec3<f32>,
    @location(3) depth: f32,
}

struct Uniforms {
    view_proj: mat4x4<f32>,
    model: mat4x4<f32>,
    time: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    
    // Apply model transformation
    let world_position = uniforms.model * vec4<f32>(model.position, 1.0);
    
    // Apply view-projection transformation
    out.clip_position = uniforms.view_proj * world_position;
    
    // Pass through texture coordinates and color
    out.tex_coords = model.tex_coords;
    out.color = model.color;
    out.world_position = world_position.xyz;
    out.depth = out.clip_position.z / out.clip_position.w;
    
    return out;
}

// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Base color from vertex
    var color = in.color;
    
    // Apply depth-based fog/fade effect
    let fog_factor = 1.0 - clamp(in.depth * 2.0, 0.0, 1.0);
    color = mix(vec4<f32>(0.1, 0.1, 0.1, 1.0), color, fog_factor);
    
    // Add subtle glow effect based on position
    let glow_intensity = 0.3 + 0.7 * (1.0 - abs(in.world_position.y));
    color = color * glow_intensity;
    
    // Add some animated shimmer
    let shimmer = 0.9 + 0.1 * sin(uniforms.time * 2.0 + in.world_position.x * 10.0);
    color = color * shimmer;
    
    // Edge highlighting for 3D effect
    let edge_factor = 1.0 - smoothstep(0.0, 0.1, min(
        min(in.tex_coords.x, 1.0 - in.tex_coords.x),
        min(in.tex_coords.y, 1.0 - in.tex_coords.y)
    ));
    color = color + vec4<f32>(0.2, 0.2, 0.4, 0.0) * edge_factor;
    
    return color;
}