// CodeSkew Unified Compute Shader
// Combines animated background effects with SDF text rendering
// Based on wgpu-compute-toy terminal-overlay example

#define TERMINAL_ROWS 30
#define TERMINAL_COLS 80

// Advanced 3D curved text rendering with Glyphon integration
fn terminal_render(pos: uint2) -> float4 {
    let screen_size = uint2(textureDimensions(screen));

    // Normalized screen coordinates
    var uv = float2(pos) / float2(screen_size);

    // SMOOTH SLOW SCROLLING - text moves down very slowly
    let scroll_speed = 0.01; // Much slower scroll
    let scroll_offset = fract(time.elapsed * scroll_speed);

    // 3D PERSPECTIVE TRANSFORMATION
    // LEFT LARGER THAN RIGHT (closer to viewer on left side)
    let perspective_strength = 0.6;
    let horizontal_depth = 1.0 + uv.x * perspective_strength; // Left side (x=0) larger, right side (x=1) smaller

    // ANGLED SKEW (text leans away from viewer like looking at screen from side)
    let skew_angle = 0.15; // Perfect subtle lean

    // 3D DIMENSIONALITY - largest at 2/3 down, folding smaller at top/bottom
    let fold_point = 0.67; // 2/3 down the screen
    let fold_strength = 0.4; // Subtle dimensionality

    var vertical_scale = 1.0;
    if (uv.y < fold_point) {
        // From top to 2/3: gradually larger
        let progress = uv.y / fold_point;
        vertical_scale = 1.0 + fold_strength * progress;
    } else {
        // From 2/3 to bottom: gradually smaller
        let progress = (uv.y - fold_point) / (1.0 - fold_point);
        vertical_scale = 1.0 + fold_strength * (1.0 - progress);
    }

    // Apply transformations
    var transformed_uv = uv;
    transformed_uv.x = (transformed_uv.x - 0.5) / horizontal_depth + 0.5; // Left larger than right perspective
    transformed_uv.y = (transformed_uv.y - 0.5) / vertical_scale + 0.5; // 3D fold effect
    transformed_uv.x -= (1.0 - transformed_uv.y) * skew_angle; // Angled skew - top leans toward viewer (correct direction)
    transformed_uv.y += scroll_offset; // Smooth scrolling

    // Sample the Glyphon-rendered text texture directly
    if (transformed_uv.x >= 0.0 && transformed_uv.x <= 1.0 &&
        transformed_uv.y >= 0.0 && transformed_uv.y <= 1.0) {

        let text_sample = textureSampleLevel(channel1, trilinear, transformed_uv, 0.);

        if (text_sample.a > 0.1) { // Text is present
            var col = float4(0);

            // Create crisp white text from Glyphon
            col = float4(1.0, 1.0, 1.0, text_sample.a);

            // Dynamic color based on 3D position and depth
            let depth_brightness = horizontal_depth * vertical_scale;
            let position_wave = sin(time.elapsed * 0.3 + transformed_uv.x * 20.0 + transformed_uv.y * 15.0);
            let color_shimmer = mix(0.85, 1.15, position_wave);

            // Enhanced color with depth and shimmer
            col.r *= depth_brightness * color_shimmer;
            col.g *= depth_brightness * color_shimmer * 0.95;
            col.b *= depth_brightness * color_shimmer * 1.1; // Slight blue enhancement

            // Depth-based transparency for 3D effect
            col.a *= mix(0.75, 1.0, depth_brightness);

            return col;
        }
    }
    return float4(0);
}

// Epic zeroshot_original background effect - the proven gorgeous one!
fn background_effect(pos: uint2) -> vec4<f32> {
    let screen_size = textureDimensions(screen);
    let frag_coord = vec2<f32>(f32(pos.x), f32(screen_size.y - pos.y));

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

    return vec4<f32>(col, 1.0);
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let screen_size = textureDimensions(screen);

    if (id.x >= screen_size.x || id.y >= screen_size.y) { return; }

    // Render animated background
    var final_color = background_effect(id.xy);

    // Overlay text rendering
    let text_color = terminal_render(id.xy);

    // Alpha blend text over background
    if (text_color.a > 0.0) {
        final_color = mix(final_color, text_color, text_color.a);
    }

    textureStore(screen, id.xy, final_color);
}
