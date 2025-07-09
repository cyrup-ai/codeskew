// Extended math utilities for WGSL
// Adapted from wgpu-compute-toy standard library

// Constants
const PI = 3.141592653589793;
const TAU = 6.283185307179586;
const E = 2.718281828459045;
const PHI = 1.618033988749895; // Golden ratio

// Extended math functions
fn saturate(x: f32) -> f32 {
    return clamp(x, 0.0, 1.0);
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    return a + (b - a) * t;
}

fn smoothstep01(x: f32) -> f32 {
    return x * x * (3.0 - 2.0 * x);
}

fn smootherstep(x: f32) -> f32 {
    return x * x * x * (x * (x * 6.0 - 15.0) + 10.0);
}

// Rotation matrices
fn rot2(a: f32) -> mat2x2<f32> {
    let c = cos(a);
    let s = sin(a);
    return mat2x2<f32>(c, -s, s, c);
}

fn rotX(a: f32) -> mat3x3<f32> {
    let c = cos(a);
    let s = sin(a);
    return mat3x3<f32>(
        1.0, 0.0, 0.0,
        0.0, c, -s,
        0.0, s, c
    );
}

fn rotY(a: f32) -> mat3x3<f32> {
    let c = cos(a);
    let s = sin(a);
    return mat3x3<f32>(
        c, 0.0, s,
        0.0, 1.0, 0.0,
        -s, 0.0, c
    );
}

fn rotZ(a: f32) -> mat3x3<f32> {
    let c = cos(a);
    let s = sin(a);
    return mat3x3<f32>(
        c, -s, 0.0,
        s, c, 0.0,
        0.0, 0.0, 1.0
    );
}

// Distance functions for raymarching
fn sdSphere(p: vec3<f32>, r: f32) -> f32 {
    return length(p) - r;
}

fn sdBox(p: vec3<f32>, b: vec3<f32>) -> f32 {
    let d = abs(p) - b;
    return length(max(d, vec3(0.0))) + min(max(d.x, max(d.y, d.z)), 0.0);
}

fn sdTorus(p: vec3<f32>, t: vec2<f32>) -> f32 {
    let q = vec2(length(p.xz) - t.x, p.y);
    return length(q) - t.y;
}

fn sdCylinder(p: vec3<f32>, h: f32, r: f32) -> f32 {
    let d = abs(vec2(length(p.xz), p.y)) - vec2(r, h);
    return min(max(d.x, d.y), 0.0) + length(max(d, vec2(0.0)));
}

// Operations
fn opUnion(d1: f32, d2: f32) -> f32 {
    return min(d1, d2);
}

fn opSubtraction(d1: f32, d2: f32) -> f32 {
    return max(-d1, d2);
}

fn opIntersection(d1: f32, d2: f32) -> f32 {
    return max(d1, d2);
}

fn opSmoothUnion(d1: f32, d2: f32, k: f32) -> f32 {
    let h = clamp(0.5 + 0.5 * (d2 - d1) / k, 0.0, 1.0);
    return mix(d2, d1, h) - k * h * (1.0 - h);
}

// UV mapping helpers
fn sphericalUV(p: vec3<f32>) -> vec2<f32> {
    let theta = atan2(p.z, p.x);
    let phi = acos(p.y / length(p));
    return vec2((theta + PI) / TAU, phi / PI);
}

fn cylindricalUV(p: vec3<f32>) -> vec2<f32> {
    let theta = atan2(p.z, p.x);
    return vec2((theta + PI) / TAU, p.y);
}

// Easing functions
fn easeInQuad(t: f32) -> f32 {
    return t * t;
}

fn easeOutQuad(t: f32) -> f32 {
    return 1.0 - (1.0 - t) * (1.0 - t);
}

fn easeInOutQuad(t: f32) -> f32 {
    if (t < 0.5) {
        return 2.0 * t * t;
    } else {
        return 1.0 - pow(-2.0 * t + 2.0, 2.0) / 2.0;
    }
}

fn easeInCubic(t: f32) -> f32 {
    return t * t * t;
}

fn easeOutCubic(t: f32) -> f32 {
    return 1.0 - pow(1.0 - t, 3.0);
}

fn easeInOutCubic(t: f32) -> f32 {
    if (t < 0.5) {
        return 4.0 * t * t * t;
    } else {
        return 1.0 - pow(-2.0 * t + 2.0, 3.0) / 2.0;
    }
}