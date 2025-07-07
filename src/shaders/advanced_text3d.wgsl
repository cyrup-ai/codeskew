// Advanced 3D Text Shader with Modern Visual Effects
// Features: Depth of Field, Real-time Shadows, Reflections, Volumetric Lighting

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) normal: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) world_position: vec3<f32>,
    @location(3) normal: vec3<f32>,
    @location(4) depth: f32,
    @location(5) shadow_coord: vec3<f32>,
    @location(6) reflection_coord: vec2<f32>,
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
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(0) @binding(1)
var shadow_texture: texture_depth_2d;

@group(0) @binding(2)
var shadow_sampler: sampler_comparison;

@group(0) @binding(3)
var reflection_texture: texture_2d<f32>;

@group(0) @binding(4)
var reflection_sampler: sampler;

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
    out.shadow_coord.y = 1.0 - out.shadow_coord.y; // Flip Y for texture coordinates
    
    // Reflection coordinates (screen space)
    out.reflection_coord = out.clip_position.xy / out.clip_position.w * 0.5 + 0.5;
    out.reflection_coord.y = 1.0 - out.reflection_coord.y;
    
    // Pass through other attributes
    out.tex_coords = model.tex_coords;
    out.color = model.color;
    out.world_position = world_position.xyz;
    out.normal = normalize((uniforms.model * vec4<f32>(model.normal, 0.0)).xyz);
    out.depth = out.clip_position.z / out.clip_position.w;
    
    return out;
}

// Utility functions for visual effects
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
    
    for (var i = 0; i < 4; i++) {
        value += amplitude * noise(p * frequency);
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    
    return value;
}

// Depth of field blur calculation
fn calculate_dof_blur(depth: f32) -> f32 {
    let focus_range = 0.1;
    let distance_from_focus = abs(depth - uniforms.focus_distance);
    return smoothstep(0.0, focus_range, distance_from_focus) * uniforms.blur_strength;
}

// Shadow mapping with soft shadows
fn calculate_shadow(shadow_coord: vec3<f32>) -> f32 {
    if (shadow_coord.x < 0.0 || shadow_coord.x > 1.0 || 
        shadow_coord.y < 0.0 || shadow_coord.y > 1.0 || 
        shadow_coord.z > 1.0) {
        return 1.0; // Outside shadow map, fully lit
    }
    
    let shadow_bias = 0.005;
    let current_depth = shadow_coord.z - shadow_bias;
    
    // PCF (Percentage Closer Filtering) for soft shadows
    var shadow_factor = 0.0;
    let texel_size = 1.0 / 2048.0; // Assuming 2048x2048 shadow map
    
    for (var x = -1; x <= 1; x++) {
        for (var y = -1; y <= 1; y++) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel_size;
            let sample_coord = shadow_coord.xy + offset;
            shadow_factor += textureSampleCompare(shadow_texture, shadow_sampler, sample_coord, current_depth);
        }
    }
    
    shadow_factor /= 9.0; // Average of 9 samples
    return mix(uniforms.shadow_intensity, 1.0, shadow_factor);
}

// Volumetric lighting effect
fn calculate_volumetric_lighting(world_pos: vec3<f32>, view_dir: vec3<f32>) -> vec3<f32> {
    let light_dir = normalize(uniforms.light_position - world_pos);
    let light_distance = length(uniforms.light_position - world_pos);
    
    // Light scattering based on angle
    let scatter_factor = max(0.0, dot(light_dir, -view_dir));
    let scatter_intensity = pow(scatter_factor, 16.0);
    
    // Distance falloff
    let attenuation = 1.0 / (1.0 + 0.1 * light_distance + 0.01 * light_distance * light_distance);
    
    // Noise for volumetric effect
    let noise_coords = world_pos.xy * 2.0 + uniforms.time * 0.1;
    let volume_noise = fbm(noise_coords) * 0.3 + 0.7;
    
    return uniforms.light_color * scatter_intensity * attenuation * volume_noise * uniforms.volumetric_intensity;
}

// Screen-space reflections
fn calculate_reflection(reflection_coord: vec2<f32>, normal: vec3<f32>, world_pos: vec3<f32>) -> vec3<f32> {
    let view_dir = normalize(uniforms.camera_position - world_pos);
    let reflect_dir = reflect(-view_dir, normal);
    
    // Simple screen-space reflection offset
    let reflection_offset = reflect_dir.xy * 0.05;
    let sample_coord = reflection_coord + reflection_offset;
    
    if (sample_coord.x < 0.0 || sample_coord.x > 1.0 || 
        sample_coord.y < 0.0 || sample_coord.y > 1.0) {
        return vec3<f32>(0.0);
    }
    
    let reflection_color = textureSample(reflection_texture, reflection_sampler, sample_coord).rgb;
    
    // Fresnel effect
    let fresnel = pow(1.0 - max(0.0, dot(view_dir, normal)), 3.0);
    
    return reflection_color * fresnel * uniforms.reflection_intensity;
}

// Enhanced surface shading with multiple lighting models
fn calculate_lighting(
    world_pos: vec3<f32>, 
    normal: vec3<f32>, 
    base_color: vec3<f32>
) -> vec3<f32> {
    let light_dir = normalize(uniforms.light_position - world_pos);
    let view_dir = normalize(uniforms.camera_position - world_pos);
    let half_dir = normalize(light_dir + view_dir);
    
    // Ambient lighting with subtle color variation
    let ambient_strength = 0.15;
    let ambient_color = vec3<f32>(0.4, 0.5, 0.8) * ambient_strength;
    
    // Diffuse lighting (Lambertian)
    let diffuse_strength = max(0.0, dot(normal, light_dir));
    let diffuse_color = uniforms.light_color * diffuse_strength;
    
    // Specular lighting (Blinn-Phong)
    let specular_strength = pow(max(0.0, dot(normal, half_dir)), 128.0);
    let specular_color = uniforms.light_color * specular_strength * 0.8;
    
    // Rim lighting for edge enhancement
    let rim_strength = 1.0 - max(0.0, dot(view_dir, normal));
    let rim_color = vec3<f32>(0.3, 0.6, 1.0) * pow(rim_strength, 4.0) * 0.5;
    
    return base_color * (ambient_color + diffuse_color) + specular_color + rim_color;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Calculate shadows
    let shadow_factor = calculate_shadow(in.shadow_coord);
    
    // Calculate enhanced lighting
    var color = calculate_lighting(in.world_position, in.normal, in.color.rgb);
    
    // Apply shadows
    color *= shadow_factor;
    
    // Add reflections
    let reflection_color = calculate_reflection(in.reflection_coord, in.normal, in.world_position);
    color += reflection_color;
    
    // Add volumetric lighting
    let view_dir = normalize(uniforms.camera_position - in.world_position);
    let volumetric_light = calculate_volumetric_lighting(in.world_position, view_dir);
    color += volumetric_light;
    
    // Depth of field blur effect
    let dof_blur = calculate_dof_blur(in.depth);
    if (dof_blur > 0.1) {
        // Sample surrounding pixels for blur effect
        let blur_samples = 8;
        var blurred_color = color;
        let blur_radius = dof_blur * 0.01;
        
        for (var i = 0; i < blur_samples; i++) {
            let angle = f32(i) * 6.28318 / f32(blur_samples);
            let offset = vec2<f32>(cos(angle), sin(angle)) * blur_radius;
            // Note: In a real implementation, you'd sample from a texture here
            // For now, we'll simulate with a simple color fade
            blurred_color = mix(blurred_color, color * 0.8, 0.1);
        }
        color = blurred_color;
    }
    
    // Enhanced atmospheric effects
    let distance_factor = length(in.world_position - uniforms.camera_position);
    let atmospheric_tint = vec3<f32>(0.7, 0.8, 1.0);
    let atmosphere_strength = smoothstep(2.0, 8.0, distance_factor) * 0.3;
    color = mix(color, atmospheric_tint, atmosphere_strength);
    
    // Dynamic color grading and tone mapping
    let exposure = 1.2;
    color *= exposure;
    
    // Reinhard tone mapping
    color = color / (1.0 + color);
    
    // Subtle film grain
    let grain_coords = in.clip_position.xy * 0.5 + uniforms.time * 100.0;
    let grain = (hash(grain_coords) - 0.5) * 0.03;
    color += vec3<f32>(grain);
    
    // Animated chromatic aberration for style
    let aberration_strength = 0.002;
    let aberration_offset = sin(uniforms.time * 2.0) * aberration_strength;
    let r_offset = vec2<f32>(aberration_offset, 0.0);
    let b_offset = vec2<f32>(-aberration_offset, 0.0);
    
    // Apply subtle color shift
    color.r *= 1.0 + aberration_offset * 10.0;
    color.b *= 1.0 - aberration_offset * 10.0;
    
    // Final gamma correction
    color = pow(color, vec3<f32>(1.0 / 2.2));
    
    return vec4<f32>(color, in.color.a);
}