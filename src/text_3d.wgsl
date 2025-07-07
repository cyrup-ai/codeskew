// Simple 3D text shader for wgpu-compute-toy
@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: uint3) {
    let dims = uint2(SCREEN_WIDTH, SCREEN_HEIGHT);
    if (id.x >= dims.x || id.y >= dims.y) { return; }
    
    let uv = vec2<f32>(id.xy) / vec2<f32>(dims);
    
    // Simple text rendering with 3D perspective
    let center = vec2<f32>(0.5, 0.5);
    let dist = distance(uv, center);
    
    // Create some text-like patterns
    let wave = sin(uv.x * 20.0 + time.elapsed * 2.0) * 0.1;
    let pattern = step(0.4, sin(uv.y * 10.0 + wave));
    
    // Apply 3D perspective effect
    let perspective = 1.0 - dist * 0.5;
    let color = vec3<f32>(0.2, 0.8, 1.0) * pattern * perspective;
    
    textureStore(pass_out, int2(id.xy), 0, vec4<f32>(color, 1.0));
}