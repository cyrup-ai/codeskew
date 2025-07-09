// Color space conversion functions for WGSL
// Adapted from wgpu-compute-toy standard library

// RGB to HSV conversion
fn rgb2hsv(c: vec3<f32>) -> vec3<f32> {
    let k = vec4(0.0, -1.0 / 3.0, 2.0 / 3.0, -1.0);
    let p = mix(vec4(c.bg, k.wz), vec4(c.gb, k.xy), step(c.b, c.g));
    let q = mix(vec4(p.xyw, c.r), vec4(c.r, p.yzx), step(p.x, c.r));
    
    let d = q.x - min(q.w, q.y);
    let e = 1.0e-10;
    return vec3(abs(q.z + (q.w - q.y) / (6.0 * d + e)), d / (q.x + e), q.x);
}

// HSV to RGB conversion
fn hsv2rgb(c: vec3<f32>) -> vec3<f32> {
    let k = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    let p = abs(fract(c.xxx + k.xyz) * 6.0 - k.www);
    return c.z * mix(k.xxx, clamp(p - k.xxx, vec3(0.0), vec3(1.0)), c.y);
}

// RGB to HSL conversion
fn rgb2hsl(c: vec3<f32>) -> vec3<f32> {
    let maxc = max(c.r, max(c.g, c.b));
    let minc = min(c.r, min(c.g, c.b));
    let l = (maxc + minc) * 0.5;
    
    if (maxc == minc) {
        return vec3(0.0, 0.0, l); // achromatic
    }
    
    let d = maxc - minc;
    var s: f32;
    if (l > 0.5) {
        s = d / (2.0 - maxc - minc);
    } else {
        s = d / (maxc + minc);
    }
    
    var h: f32;
    if (maxc == c.r) {
        h = (c.g - c.b) / d + select(6.0, 0.0, c.g >= c.b);
    } else if (maxc == c.g) {
        h = (c.b - c.r) / d + 2.0;
    } else {
        h = (c.r - c.g) / d + 4.0;
    }
    h /= 6.0;
    
    return vec3(h, s, l);
}

// HSL to RGB conversion
fn hsl2rgb(c: vec3<f32>) -> vec3<f32> {
    let h = c.x;
    let s = c.y;
    let l = c.z;
    
    if (s == 0.0) {
        return vec3(l); // achromatic
    }
    
    let q = select(l * (1.0 + s), l + s - l * s, l < 0.5);
    let p = 2.0 * l - q;
    
    return vec3(
        hue2rgb(p, q, h + 1.0 / 3.0),
        hue2rgb(p, q, h),
        hue2rgb(p, q, h - 1.0 / 3.0)
    );
}

fn hue2rgb(p: f32, q: f32, t_in: f32) -> f32 {
    var t = t_in;
    if (t < 0.0) { t += 1.0; }
    if (t > 1.0) { t -= 1.0; }
    if (t < 1.0 / 6.0) { return p + (q - p) * 6.0 * t; }
    if (t < 1.0 / 2.0) { return q; }
    if (t < 2.0 / 3.0) { return p + (q - p) * (2.0 / 3.0 - t) * 6.0; }
    return p;
}

// sRGB gamma correction
fn linearToSRGB(c: vec3<f32>) -> vec3<f32> {
    return pow(c, vec3(1.0 / 2.2));
}

fn sRGBToLinear(c: vec3<f32>) -> vec3<f32> {
    return pow(c, vec3(2.2));
}

// More accurate sRGB conversion
fn linearToSRGBAccurate(c: vec3<f32>) -> vec3<f32> {
    return select(
        1.055 * pow(c, vec3(1.0 / 2.4)) - 0.055,
        12.92 * c,
        c <= vec3(0.0031308)
    );
}

fn sRGBToLinearAccurate(c: vec3<f32>) -> vec3<f32> {
    return select(
        pow((c + 0.055) / 1.055, vec3(2.4)),
        c / 12.92,
        c <= vec3(0.04045)
    );
}

// Color temperature (Kelvin to RGB)
fn kelvinToRGB(kelvin: f32) -> vec3<f32> {
    let temp = clamp(kelvin, 1000.0, 40000.0) / 100.0;
    var r: f32;
    var g: f32;
    var b: f32;
    
    if (temp <= 66.0) {
        r = 255.0;
        g = temp;
        g = 99.4708025861 * log(g) - 161.1195681661;
        
        if (temp >= 19.0) {
            b = temp - 10.0;
            b = 138.5177312231 * log(b) - 305.0447927307;
        } else {
            b = 0.0;
        }
    } else {
        r = temp - 60.0;
        r = 329.698727446 * pow(r, -0.1332047592);
        
        g = temp - 60.0;
        g = 288.1221695283 * pow(g, -0.0755148492);
        
        b = 255.0;
    }
    
    return clamp(vec3(r, g, b) / 255.0, vec3(0.0), vec3(1.0));
}

// Palette generation
fn palette(t: f32, a: vec3<f32>, b: vec3<f32>, c: vec3<f32>, d: vec3<f32>) -> vec3<f32> {
    return a + b * cos(6.28318 * (c * t + d));
}

// Common color palettes
fn sunset(t: f32) -> vec3<f32> {
    return palette(t, vec3(0.5, 0.5, 0.5), vec3(0.5, 0.5, 0.5), vec3(1.0, 1.0, 1.0), vec3(0.0, 0.33, 0.67));
}

fn ocean(t: f32) -> vec3<f32> {
    return palette(t, vec3(0.5, 0.5, 0.5), vec3(0.5, 0.5, 0.5), vec3(1.0, 1.0, 0.5), vec3(0.8, 0.9, 0.3));
}

fn fire(t: f32) -> vec3<f32> {
    return palette(t, vec3(0.5, 0.5, 0.5), vec3(0.5, 0.5, 0.5), vec3(1.0, 0.7, 0.4), vec3(0.0, 0.15, 0.20));
}