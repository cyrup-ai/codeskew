// 3D voxel simulation rendering with enhanced visuals and interactivity
// Constants
const REZ: u32 = 192u;             // Voxel grid resolution
const THREADS: u32 = 64u;          // Threads per workgroup
const CUBE_SIZE: f32 = 1.0;        // Simulation cube size
const MAX_STEPS: u32 = REZ * 2u;   // Max raymarching steps
const STEP_SIZE: f32 = 1.0 / f32(REZ); // Base step size
const DENSITY_THRESHOLD: f32 = 0.99;   // Early termination threshold
const VISCOSITY: f32 = 0.05;       // Fluid viscosity
const DIFFUSION: f32 = 0.1;        // Density diffusion rate

// Storage buffers
@group(0) @binding(0) var<storage, read_write> D: array<vec2<f32>, REZ * REZ * REZ * 2u>; // Double-buffered voxel data
@group(0) @binding(1) var<storage, read_write> N: array<vec2<f32>, REZ * REZ * REZ>;     // Normalization buffer
@group(0) @binding(2) var screen: texture_storage_2d<rgba8unorm, write>;                // Output texture
@group(0) @binding(3) var<uniform> time: Time;                                          // Time info
@group(0) @binding(4) var<uniform> mouse: Mouse;                                        // Mouse info
@group(0) @binding(5) var<uniform> keyboard: Keyboard;                                  // Keyboard input

// Utility functions
fn c13(a: u32) -> vec3<u32> {  // 1D to 3D coordinates
    return vec3<u32>(a % REZ, (a / REZ) % REZ, a / (REZ * REZ));
}

fn c31(a: vec3<u32>) -> u32 {  // 3D to 1D coordinates
    return a.x + a.y * REZ + a.z * REZ * REZ;
}

fn mod3(a: vec3<f32>, b: f32) -> vec3<f32> {  // Periodic boundary modulo
    return fract(a / b) * b;
}

fn lerp_voxel(p: vec3<f32>) -> vec2<f32> {  // Trilinear interpolation of voxel data
    let fw = ((time.frame + 0u) & 1u) * REZ * REZ * REZ; // Current write buffer
    let i = vec3<u32>(floor(p));
    let f = fract(p);
    // Clamp indices to avoid out-of-bounds access
    let i_clamped = vec3<u32>(
        min(i.x, REZ - 1u),
        min(i.y, REZ - 1u),
        min(i.z, REZ - 1u)
    );
    // Sample 8 corners
    let v000 = D[fw + c31(i_clamped)];
    let v100 = D[fw + c31(vec3<u32>(min(i_clamped.x + 1u, REZ - 1u), i_clamped.y, i_clamped.z))];
    let v010 = D[fw + c31(vec3<u32>(i_clamped.x, min(i_clamped.y + 1u, REZ - 1u), i_clamped.z))];
    let v110 = D[fw + c31(vec3<u32>(min(i_clamped.x + 1u, REZ - 1u), min(i_clamped.y + 1u, REZ - 1u), i_clamped.z))];
    let v001 = D[fw + c31(vec3<u32>(i_clamped.x, i_clamped.y, min(i_clamped.z + 1u, REZ - 1u)))];
    let v101 = D[fw + c31(vec3<u32>(min(i_clamped.x + 1u, REZ - 1u), i_clamped.y, min(i_clamped.z + 1u, REZ - 1u)))];
    let v011 = D[fw + c31(vec3<u32>(i_clamped.x, min(i_clamped.y + 1u, REZ - 1u), min(i_clamped.z + 1u, REZ - 1u)))];
    let v111 = D[fw + c31(vec3<u32>(min(i_clamped.x + 1u, REZ - 1u), min(i_clamped.y + 1u, REZ - 1u), min(i_clamped.z + 1u, REZ - 1u)))];
    // Trilinear interpolation
    return mix(
        mix(
            mix(v000, v100, f.x),
            mix(v010, v110, f.x),
            f.y
        ),
        mix(
            mix(v001, v101, f.x),
            mix(v011, v111, f.x),
            f.y
        ),
        f.z
    );
}

// Rendering shader: Volumetric raymarching with enhanced visuals
@compute @workgroup_size(8, 8, 1)
fn main_image(@builtin(global_invocation_id) id: vec3<u32>) {
    let screen_size = textureDimensions(screen);
    if (id.x >= screen_size.x || id.y >= screen_size.y) { return; }
    let fragCoord = vec3<f32>(id) + 0.5;
    let iResolution = vec2<f32>(screen_size);
    let iTime = select(time.elapsed, time.elapsed * 0.0, keyboard.pause || mouse.click > 0.0); // Pause during interaction
    let fw = ((time.frame + 0u) & 1u) * REZ * REZ * REZ;

    // Camera with smooth zoom
    let mouse_pos = vec2<f32>(mouse.pos);
    let m = (2.0 * mouse_pos - iResolution) / iResolution.y;
    let zoom_target = clamp(2.0 + keyboard.zoom * 0.1, 1.0, 5.0);
    var zoom = mix(zoom, zoom_target, 0.1); // Smooth zoom transition
    var camPos = vec3<f32>(0.0, 0.0, 3.0 * zoom);
    if (mouse.click > 0.0) {
        camPos = vec3<f32>(cos(m.x) * zoom, m.y * zoom, sin(m.x) * zoom) * 2.0;
    } else {
        camPos = vec3<f32>(cos(iTime * 0.1) * zoom, sin(iTime * 0.2) * 0.3, sin(iTime * 0.15) * zoom) * 2.0;
    }
    let camDir = normalize(-camPos);

    // Ray setup
    let u = (2.0 * fragCoord.xy - iResolution) / iResolution.y;
    let mtx0 = normalize(vec3<f32>(camDir.z, 0.0, -camDir.x));
    let mtx = mat3x3<f32>(mtx0, cross(camDir, mtx0), camDir);
    let ray = mtx * normalize(vec3<f32>(u, 2.0));
    let ray_inv = 1.0 / ray;
    let ray_sign = sign(ray) * 0.5 + 0.5;

    // Ray-box intersection
    let tMin = (vec3<f32>(-CUBE_SIZE) - camPos) * ray_inv;
    let tMax = (vec3<f32>(CUBE_SIZE) - camPos) * ray_inv;
    let t1 = min(tMin, tMax);
    let t2 = max(tMin, tMax);
    let tN = max(max(t1.x, t1.y), t1.z);
    let tF = min(min(t2.x, t2.y), t2.z);
    let inside = all(camPos >= vec3<f32>(-CUBE_SIZE)) && all(camPos <= vec3<f32>(CUBE_SIZE));
    let tF_valid = select(1.0, 0.0, tF < tN);

    // Raymarching
    var p = camPos + ray * tN * select(1.001, 0.0, inside);
    p = (p * 0.5 + 0.5) * f32(REZ);
    var lig = vec4<f32>(1.0);
    var rif = vec4<f32>(0.0);
    let light_dir = normalize(vec3<f32>(1.0, 1.0, -1.0));
    var accumulated_density = 0.0;

    for (var i: u32 = 0u; i < MAX_STEPS; i++) {
        if (any(abs(p / f32(REZ) * 2.0 - 1.0) >= vec3<f32>(1.0)) || accumulated_density > DENSITY_THRESHOLD) { break; }
        let t = lerp_voxel(p);
        let density = clamp(dot(t, vec2<f32>(1.0, -1.0)) * 6.0, 0.0, 1.0);

        // Volumetric lighting with glow
        let shadow = lerp_voxel(p + light_dir * 2.0).x * 0.5 + 0.5;
        let ambient = 0.2 + 0.8 * exp(-density * 0.1);
        let scatter = 0.3 * density * shadow;
        let glow = 0.1 * pow(density, 2.0); // Subtle glow effect

        // Smooth color gradient (blue -> cyan -> magenta)
        let col = mix(
            mix(vec4<f32>(0.1, 0.2, 0.8, 0.0), vec4<f32>(0.1, 0.8, 0.8, 0.0), density),
            vec4<f32>(0.8, 0.1, 0.8, 0.0),
            density
        );
        let atten = exp(-density * STEP_SIZE * 0.5);
        rif += lig * (col * scatter + vec4<f32>(glow, glow, glow, 0.0)) * (1.0 - atten);
        lig *= atten * vec4<f32>(ambient, ambient, ambient, 1.0);
        accumulated_density += density * STEP_SIZE;

        // Adaptive stepping
        let dist_factor = length(p - camPos) / f32(REZ);
        let step = max(STEP_SIZE, STEP_SIZE / (density + 0.01) * (1.0 + dist_factor));
        p += ray * step * f32(REZ);
    }

    // Background with depth-based fog
    let bg = vec4<f32>(0.02, 0.02, 0.05, 1.0) * (1.0 + 0.5 * sin(dot(ray.xy, ray.xy) * 100.0));
    let fog_factor = exp(-tF * 0.01);
    let final_color = mix(bg * fog_factor, rif, tF_valid);

    // Temporal anti-aliasing
    let prev_color = textureLoad(screen, vec2<i32>(id.xy)).rgba;
    let taa_color = mix(prev_color, final_color, 0.1);

    textureStore(screen, vec2<i32>(id.xy), taa_color);
}
