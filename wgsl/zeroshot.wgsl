@compute @workgroup_size(16, 16)
fn main_image(@builtin(global_invocation_id) id: vec3<u32>) {
    // Viewport resolution (in pixels)
    let screen_size = textureDimensions(screen);

    // Prevent overdraw for workgroups on the edge of the viewport
    if (id.x >= screen_size.x || id.y >= screen_size.y) { return; }

    let frag_coord = vec2<f32>(f32(id.x), f32(screen_size.y - id.y));

    let t = time.elapsed * 0.1;
    let rmat = mat2x2<f32>(cos(t), cos(t + 33.), cos(t + 11.), cos(t));
    var d = vec2<f32>(frag_coord.x - 2. * frag_coord.y + 0.2 * f32(screen_size.y)) / 2e3;
    var col = vec3<f32>(0.0);
    for (var i = 1.0; i < 16.; i += 1. / i) {
        let uv = (frag_coord + i * d) * mat2x2<f32>(2., 1., -2., 4.) / f32(screen_size.y) - vec2<f32>(-.1, .6);
        let le = length(uv) / vec2<f32>(3., 8.);

        d = d * mat2x2<f32>(-73., -67., 67., -73.) / 99.;

        var dust = textureSampleLevel(channel0, bilinear_repeat, uv * rmat, 0.).rgb;
        // fix: convert rgb from inverse-gamma-space to linear-space
        dust = pow(dust, vec3<f32>(1.0 / 2.2));

        var ring = textureSampleLevel(channel0, bilinear_repeat, le, 0.).rgb;
        // fix: convert rgb from inverse-gamma-space to linear-space
        ring = pow(ring, vec3<f32>(1.0 / 2.2));

        col += pow(0.33 * ring * dust / le.x, vec3<f32>(5., 8., 9.));
    }

    col = pow(col, vec3<f32>(0.5));

    // Output to screen (linear colour space)
    textureStore(screen, id.xy, vec4<f32>(col, 1.));
}
