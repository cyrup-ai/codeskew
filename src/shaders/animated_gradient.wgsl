// Animated gradient shader for background effects
// This shader creates a dynamic, animated gradient that shifts colors over time

struct TimeUniforms {
    time_elapsed: f32,
    time_delta: f32,
    resolution: vec2<f32>,
}

@group(0) @binding(0) var<uniform> time: TimeUniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// Vertex shader - creates a fullscreen triangle
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    // Generate a fullscreen triangle
    // vertex 0: (-1, -1)
    // vertex 1: ( 3, -1)
    // vertex 2: (-1,  3)
    let x = f32(i32(vertex_index) - 1) * 2.0;
    let y = f32(i32(vertex_index & 1u) * 4 - 1);

    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>(x * 0.5 + 0.5, 1.0 - (y * 0.5 + 0.5));

    return out;
}

// Helper function for smooth color interpolation
fn hsv_to_rgb(hsv: vec3<f32>) -> vec3<f32> {
    let h = hsv.x;
    let s = hsv.y;
    let v = hsv.z;

    let c = v * s;
    let x = c * (1.0 - abs((h * 6.0) % 2.0 - 1.0));
    let m = v - c;

    var rgb: vec3<f32>;
    if h < 1.0 / 6.0 {
        rgb = vec3<f32>(c, x, 0.0);
    } else if h < 2.0 / 6.0 {
        rgb = vec3<f32>(x, c, 0.0);
    } else if h < 3.0 / 6.0 {
        rgb = vec3<f32>(0.0, c, x);
    } else if h < 4.0 / 6.0 {
        rgb = vec3<f32>(0.0, x, c);
    } else if h < 5.0 / 6.0 {
        rgb = vec3<f32>(x, 0.0, c);
    } else {
        rgb = vec3<f32>(c, 0.0, x);
    }

    return rgb + vec3<f32>(m);
}

// Smooth noise function
fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);

    let a = fract(sin(dot(i + vec2<f32>(0.0, 0.0), vec2<f32>(12.9898, 78.233))) * 43758.5453);
    let b = fract(sin(dot(i + vec2<f32>(1.0, 0.0), vec2<f32>(12.9898, 78.233))) * 43758.5453);
    let c = fract(sin(dot(i + vec2<f32>(0.0, 1.0), vec2<f32>(12.9898, 78.233))) * 43758.5453);
    let d = fract(sin(dot(i + vec2<f32>(1.0, 1.0), vec2<f32>(12.9898, 78.233))) * 43758.5453);

    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// Fragment shader - creates animated gradient
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Normalize coordinates
    let coord = in.uv * time.resolution / min(time.resolution.x, time.resolution.y);

    // Create multiple gradient waves
    let wave1 = sin(coord.x * 3.0 + time.time_elapsed * 0.5) * 0.3;
    let wave2 = cos(coord.y * 2.0 - time.time_elapsed * 0.3) * 0.3;
    let wave3 = sin(length(coord - vec2<f32>(0.5)) * 5.0 - time.time_elapsed) * 0.2;

    // Combine waves
    let pattern = wave1 + wave2 + wave3;

    // Add some noise for texture
    let n = noise(coord * 10.0 + vec2<f32>(time.time_elapsed * 0.1));

    // Create color based on position and time
    let hue1 = fract(time.time_elapsed * 0.05 + coord.x * 0.3 + pattern * 0.1);
    let hue2 = fract(time.time_elapsed * 0.03 - coord.y * 0.3 + pattern * 0.1);

    // Mix two colors
    let color1 = hsv_to_rgb(vec3<f32>(hue1, 0.8, 0.9));
    let color2 = hsv_to_rgb(vec3<f32>(hue2, 0.7, 0.8));

    // Blend colors based on position and noise
    let blend_factor = 0.5 + 0.5 * sin(coord.x * 2.0 + coord.y * 2.0 + time.time_elapsed * 0.2);
    var final_color = mix(color1, color2, blend_factor);

    // Add subtle noise overlay
    final_color = final_color + vec3<f32>(n * 0.05);

    // Apply a subtle vignette effect
    let vignette = 1.0 - length(in.uv - vec2<f32>(0.5)) * 0.5;
    final_color = final_color * vignette;

    return vec4<f32>(final_color, 1.0);
}
