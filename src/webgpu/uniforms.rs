//! Shader uniform structures for WebGPU rendering

use bytemuck;

/// Shader uniform buffer layout
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShaderUniforms {
    pub time: f32,
    pub time_delta: f32,
    pub frame: f32,
    pub mouse: [f32; 4], // x, y, click_x, click_y
    pub resolution: [f32; 2],
    pub padding: [f32; 2],
}

impl ShaderUniforms {
    pub fn new() -> Self {
        Self {
            time: 0.0,
            time_delta: 0.016,
            frame: 0.0,
            mouse: [0.0; 4],
            resolution: [1920.0, 1080.0],
            padding: [0.0; 2],
        }
    }

    pub fn update(&mut self, time: f32, delta_time: f32) {
        self.time = time;
        self.time_delta = delta_time;
        self.frame += 1.0;
    }

    pub fn update_mouse(&mut self, x: f32, y: f32, clicked_x: f32, clicked_y: f32) {
        self.mouse = [x, y, clicked_x, clicked_y];
    }

    pub fn update_resolution(&mut self, width: f32, height: f32) {
        self.resolution = [width, height];
    }
}

impl Default for ShaderUniforms {
    fn default() -> Self {
        Self::new()
    }
}
