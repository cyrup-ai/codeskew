// Glyphon-based 3D Text Shader with Modern Visual Effects
// Features: High-quality text rendering, 3D extrusion, shadows, reflections, DOF

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) normal: vec3<f32>,
    @location(4) depth_offset: f32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) world_position: vec3<f32>,
    @location(3) normal: vec3<f32>,
    @location(4) depth: f32,
    @location(5) shadow_coord: vec3<f32>,
    @location(6) screen_coords: vec2<f32>,
    @location(7) depth_offset: f32,
}

struct Uniforms {
    view_proj: mat4x4<f32>,
    model: mat4x4<f32>,
    light_view_proj: mat4x4<f32>,
    camera_position: vec3<f32>,
    light_position: vec3<f32>,
    light_color: vec3<f32>,
    time: f32,
    focus_distance: f32,
    blur_strength: f32,
    shadow_intensity: f32,
    reflection_intensity: f32,
    volumetric_intensity: f32,
    text_depth: f32,
    extrusion_depth: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(0) @binding(1)
var shadow_texture: texture_depth_2d;

@group(0) @binding(2)
var shadow_sampler: sampler_comparison;

@group(0) @binding(3)
var text_texture: texture_2d<f32>;

@group(0) @binding(4)
var text_sampler: sampler;

@group(0) @binding(5)
var noise_texture: texture_2d<f32>;

@group(0) @binding(6)
var noise_sampler: sampler;

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    // Apply model transformation
    let world_position = uniforms.model * vec4<f32>(model.position, 1.0);
    
    // Apply view-projection transformation
    out.clip_position = uniforms.view_proj * world_position;
    
    // Shadow mapping coordinates
    let shadow_pos = uniforms.light_view_proj * world_position;
    out.shadow_coord = shadow_pos.xyz / shadow_pos.w;
    out.shadow_coord = out.shadow_coord * 0.5 + 0.5;
    out.shadow_coord.y = 1.0 - out.shadow_coord.y;
    
    // Screen space coordinates for effects
    out.screen_coords = out.clip_position.xy / out.clip_position.w * 0.5 + 0.5;
    out.screen_coords.y = 1.0 - out.screen_coords.y;
    
    // Pass through other attributes
    out.tex_coords = model.tex_coords;
    out.color = model.color;
    out.world_position = world_position.xyz;
    out.normal = normalize((uniforms.model * vec4<f32>(model.normal, 0.0)).xyz);
    out.depth = out.clip_position.z / out.clip_position.w;
    out.depth_offset = model.depth_offset;
    
    return out;
}

// Enhanced utility functions for visual effects
fn hash(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    
    return mix(
        mix(hash(i + vec2<f32>(0.0, 0.0)), hash(i + vec2<f32>(1.0, 0.0)), u.x),
        mix(hash(i + vec2<f32>(0.0, 1.0)), hash(i + vec2<f32>(1.0, 1.0)), u.x),
        u.y
    );
}

fn fbm(p: vec2<f32>) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    
    for (var i = 0; i < 5; i++) {
        value += amplitude * noise(p * frequency);
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    
    return value;
}

// Advanced depth of field calculation
fn calculate_dof_blur(depth: f32, depth_offset: f32) -> f32 {
    let adjusted_depth = depth + depth_offset * 0.1;
    let focus_range = 0.15;
    let distance_from_focus = abs(adjusted_depth - uniforms.focus_distance);
    return smoothstep(0.0, focus_range, distance_from_focus) * uniforms.blur_strength;
}

// Enhanced shadow mapping with soft shadows
fn calculate_shadow(shadow_coord: vec3<f32>) -> f32 {
    if (shadow_coord.x < 0.0 || shadow_coord.x > 1.0 || 
        shadow_coord.y < 0.0 || shadow_coord.y > 1.0 || 
        shadow_coord.z > 1.0) {
        return 1.0;
    }
    
    let shadow_bias = 0.003;
    let current_depth = shadow_coord.z - shadow_bias;
    
    // Enhanced PCF with wider sampling for softer shadows
    var shadow_factor = 0.0;
    let texel_size = 1.0 / 2048.0;
    let pcf_radius = 2.0;
    let sample_count = 25.0; // 5x5 sampling
    
    for (var x = -2; x <= 2; x++) {
        for (var y = -2; y <= 2; y++) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel_size * pcf_radius;
            let sample_coord = shadow_coord.xy + offset;
            shadow_factor += textureSampleCompare(shadow_texture, shadow_sampler, sample_coord, current_depth);
        }
    }
    
    shadow_factor /= sample_count;
    return mix(uniforms.shadow_intensity, 1.0, shadow_factor);
}

// Enhanced volumetric lighting with noise
fn calculate_volumetric_lighting(world_pos: vec3<f32>, view_dir: vec3<f32>) -> vec3<f32> {
    let light_dir = normalize(uniforms.light_position - world_pos);
    let light_distance = length(uniforms.light_position - world_pos);
    
    // Enhanced light scattering
    let scatter_factor = max(0.0, dot(light_dir, -view_dir));
    let scatter_intensity = pow(scatter_factor, 24.0);
    
    // Distance falloff with enhanced curves
    let attenuation = 1.0 / (1.0 + 0.05 * light_distance + 0.005 * light_distance * light_distance);
    
    // Multi-octave noise for more realistic volumetrics
    let noise_coords = world_pos.xy * 3.0 + uniforms.time * 0.05;
    let volume_noise = fbm(noise_coords) * 0.4 + 0.6;
    
    // Add animated dust particles effect
    let dust_coords = world_pos.xy * 8.0 + uniforms.time * 0.2;
    let dust_noise = noise(dust_coords) * 0.2 + 0.8;
    
    return uniforms.light_color * scatter_intensity * attenuation * volume_noise * dust_noise * uniforms.volumetric_intensity;
}

// Enhanced surface shading with better material properties
fn calculate_enhanced_lighting(
    world_pos: vec3<f32>, 
    normal: vec3<f32>, 
    base_color: vec3<f32>,
    depth_offset: f32
) -> vec3<f32> {
    let light_dir = normalize(uniforms.light_position - world_pos);
    let view_dir = normalize(uniforms.camera_position - world_pos);
    let half_dir = normalize(light_dir + view_dir);
    
    // Enhanced ambient with color temperature variation
    let ambient_strength = 0.12;
    let ambient_warm = vec3<f32>(0.8, 0.6, 0.4);
    let ambient_cool = vec3<f32>(0.4, 0.5, 0.8);
    let ambient_mix = mix(ambient_cool, ambient_warm, (sin(uniforms.time * 0.5) + 1.0) * 0.5);
    let ambient_color = ambient_mix * ambient_strength;
    
    // Enhanced diffuse with subsurface scattering simulation
    let diffuse_strength = max(0.0, dot(normal, light_dir));
    let subsurface = max(0.0, dot(-normal, light_dir)) * 0.3;
    let total_diffuse = diffuse_strength + subsurface;
    let diffuse_color = uniforms.light_color * total_diffuse;
    
    // Enhanced specular with roughness variation based on depth
    let roughness = 0.1 + abs(depth_offset) * 0.2;
    let specular_power = mix(256.0, 64.0, roughness);
    let specular_strength = pow(max(0.0, dot(normal, half_dir)), specular_power);
    let fresnel = pow(1.0 - max(0.0, dot(view_dir, normal)), 3.0);
    let specular_color = uniforms.light_color * specular_strength * fresnel * 0.9;
    
    // Enhanced rim lighting with color variation
    let rim_strength = 1.0 - max(0.0, dot(view_dir, normal));
    let rim_power = mix(2.0, 6.0, abs(depth_offset));
    let rim_color = mix(
        vec3<f32>(0.3, 0.6, 1.0), 
        vec3<f32>(1.0, 0.4, 0.2), 
        abs(depth_offset)
    ) * pow(rim_strength, rim_power) * 0.6;
    
    // Depth-based color variation for 3D effect
    let depth_tint = mix(vec3<f32>(1.0), vec3<f32>(0.7, 0.8, 1.0), abs(depth_offset) * 2.0);
    
    return base_color * depth_tint * (ambient_color + diffuse_color) + specular_color + rim_color;
}

// Advanced text sampling with anti-aliasing
fn sample_text_with_aa(coords: vec2<f32>) -> vec4<f32> {
    let dims = textureDimensions(text_texture);
    let texel_size = 1.0 / vec2<f32>(f32(dims.x), f32(dims.y));
    
    // Multi-tap sampling for better anti-aliasing - unrolled for WGSL compatibility
    var color = vec4<f32>(0.0);
    
    // Sample 1
    color += textureSample(text_texture, text_sampler, coords + vec2<f32>(-0.5, -0.5) * texel_size);
    // Sample 2  
    color += textureSample(text_texture, text_sampler, coords + vec2<f32>(0.5, -0.5) * texel_size);
    // Sample 3
    color += textureSample(text_texture, text_sampler, coords + vec2<f32>(-0.5, 0.5) * texel_size);
    // Sample 4
    color += textureSample(text_texture, text_sampler, coords + vec2<f32>(0.5, 0.5) * texel_size);
    
    return color * 0.25;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample text with enhanced anti-aliasing
    let text_color = sample_text_with_aa(in.screen_coords);
    
    // Skip rendering if no text at this position (for optimization)
    if (text_color.a < 0.01) {
        discard;
    }
    
    // Calculate shadows
    let shadow_factor = calculate_shadow(in.shadow_coord);
    
    // Use text alpha to modulate base color
    var base_color = in.color.rgb * text_color.rgb;
    
    // Calculate enhanced lighting
    var color = calculate_enhanced_lighting(in.world_position, in.normal, base_color, in.depth_offset);
    
    // Apply shadows
    color *= shadow_factor;
    
    // Add volumetric lighting
    let view_dir = normalize(uniforms.camera_position - in.world_position);
    let volumetric_light = calculate_volumetric_lighting(in.world_position, view_dir);
    color += volumetric_light;
    
    // Enhanced depth of field
    let dof_blur = calculate_dof_blur(in.depth, in.depth_offset);
    if (dof_blur > 0.1) {
        let blur_intensity = min(dof_blur, 1.0);
        color = mix(color, color * 0.7, blur_intensity * 0.5);
    }
    
    // Enhanced atmospheric effects with depth layers
    let distance_factor = length(in.world_position - uniforms.camera_position);
    let atmospheric_tint = vec3<f32>(0.6, 0.7, 1.0);
    let atmosphere_strength = smoothstep(3.0, 10.0, distance_factor) * 0.4;
    color = mix(color, atmospheric_tint, atmosphere_strength);
    
    // Advanced tone mapping with dynamic exposure
    let luminance = dot(color, vec3<f32>(0.299, 0.587, 0.114));
    let exposure = mix(1.0, 1.5, smoothstep(0.1, 0.8, luminance));
    color *= exposure;
    
    // Enhanced Reinhard tone mapping
    color = color / (1.0 + color * 0.8);
    
    // Film grain with temporal variation
    let grain_coords = in.clip_position.xy * 0.7 + uniforms.time * 150.0;
    let grain = (hash(grain_coords) - 0.5) * 0.025;
    color += vec3<f32>(grain);
    
    // Dynamic chromatic aberration based on distance from center
    let center_distance = length(in.screen_coords - vec2<f32>(0.5));
    let aberration_strength = center_distance * 0.003;
    let aberration_offset = sin(uniforms.time * 1.5) * aberration_strength;
    
    color.r *= 1.0 + aberration_offset * 8.0;
    color.b *= 1.0 - aberration_offset * 8.0;
    
    // Enhanced color grading
    color = pow(color, vec3<f32>(0.9)); // Slight contrast boost
    color = mix(color, color * color, 0.1); // Subtle saturation boost
    
    // Final gamma correction
    color = pow(color, vec3<f32>(1.0 / 2.2));
    
    // Preserve text alpha for proper blending
    return vec4<f32>(color, text_color.a * in.color.a);
}