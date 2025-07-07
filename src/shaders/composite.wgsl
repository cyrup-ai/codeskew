// Composite shader for blending animated background with text
// This shader renders a fullscreen quad with the background texture

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

// Vertex shader - generates a fullscreen triangle
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    // Generate positions and texture coordinates for a fullscreen triangle
    // This uses the vertex_index to create a triangle that covers the entire screen
    // when rendered with 6 vertices (two triangles)
    let x = f32((vertex_index & 1u) << 2u);
    let y = f32((vertex_index & 2u) << 1u);

    out.position = vec4<f32>(x - 1.0, y - 1.0, 0.0, 1.0);
    out.tex_coords = vec2<f32>(x * 0.5, 1.0 - (y * 0.5));

    return out;
}

// Fragment shader bindings
@group(0) @binding(0) var background_texture: texture_2d<f32>;
@group(0) @binding(1) var background_sampler: sampler;

// Fragment shader - samples the background texture
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample the background texture
    let color = textureSample(background_texture, background_sampler, in.tex_coords);

    // Return the color with full opacity
    // The text renderer will blend on top of this
    return vec4<f32>(color.rgb, 1.0);
}
