@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3u) {
    // Viewport resolution (in pixels)
    let screen_size = textureDimensions(screen);
    
    // Prevent overdraw for workgroups on the edge of the viewport
    if (id.x >= screen_size.x || id.y >= screen_size.y) { return; }

    let uv = vec2f(f32(id.x), f32(id.y)) / vec2f(f32(screen_size.x), f32(screen_size.y));
    
    // Sample directly from channel0 and output it
    let sampled = textureSampleLevel(channel0, bilinear, uv, 0.).rgb;
    
    // Output sampled color directly to screen
    textureStore(screen, id.xy, vec4f(sampled, 1.0));
}