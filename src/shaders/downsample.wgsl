// High-quality downsampling shader for 3x to 1x reduction
// Uses a Lanczos filter for superior quality

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

struct DownsampleUniforms {
    supersampling_factor: f32,
    _padding: vec3<f32>,  // Padding to 16 bytes
}

@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
@group(0) @binding(2) var<uniform> uniforms: DownsampleUniforms;

// Vertex shader - generates a fullscreen triangle
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    
    // Generate fullscreen triangle
    let x = f32((vertex_index & 1u) << 1u);
    let y = f32((vertex_index & 2u));
    
    out.position = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    out.uv = vec2<f32>(x, y);
    
    return out;
}

// Lanczos kernel function
fn lanczos(x: f32, a: f32) -> f32 {
    if (abs(x) < 0.0001) {
        return 1.0;
    }
    if (abs(x) >= a) {
        return 0.0;
    }
    
    let pi_x = x * 3.14159265359;
    let pi_x_over_a = pi_x / a;
    
    return (sin(pi_x) / pi_x) * (sin(pi_x_over_a) / pi_x_over_a);
}

// Fragment shader - performs high-quality downsampling
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let dimensions = textureDimensions(input_texture);
    let texel_size = 1.0 / vec2<f32>(dimensions);
    
    // Lanczos filter with a = 3 (6x6 kernel for 3x downsampling)
    let filter_size = 3;
    var color = vec4<f32>(0.0);
    var weight_sum = 0.0;
    
    // Sample in a 6x6 grid centered at the current pixel
    for (var y = -filter_size; y <= filter_size; y = y + 1) {
        for (var x = -filter_size; x <= filter_size; x = x + 1) {
            let offset = vec2<f32>(f32(x), f32(y));
            let sample_pos = in.uv + offset * texel_size / uniforms.supersampling_factor;
            
            // Calculate Lanczos weight
            let dist = length(offset / uniforms.supersampling_factor);
            let weight = lanczos(dist, uniforms.supersampling_factor);
            
            if (weight > 0.0) {
                // Sample with clamping to edge
                let clamped_pos = clamp(sample_pos, vec2<f32>(0.0), vec2<f32>(1.0));
                let sample = textureSample(input_texture, texture_sampler, clamped_pos);
                
                // Apply gamma correction for perceptually correct downsampling
                let linear_sample = pow(sample, vec4<f32>(2.2));
                color += linear_sample * weight;
                weight_sum += weight;
            }
        }
    }
    
    // Normalize and apply inverse gamma
    color = color / weight_sum;
    color = pow(color, vec4<f32>(1.0 / 2.2));
    
    // Ensure alpha is preserved correctly
    color.a = clamp(color.a, 0.0, 1.0);
    
    return color;
}