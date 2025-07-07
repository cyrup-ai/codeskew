//! Command buffer state management for WebGPU rendering

/// Command buffer state for reuse
pub struct CommandBufferState {
    encoder: Option<wgpu::CommandEncoder>,
    label: &'static str,
}

impl CommandBufferState {
    #[inline]
    pub fn new(label: &'static str) -> Self {
        Self {
            encoder: None,
            label,
        }
    }

    #[inline]
    pub fn get_or_create(&mut self, device: &wgpu::Device) -> &mut wgpu::CommandEncoder {
        self.encoder.get_or_insert_with(|| {
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some(self.label),
            })
        })
    }

    #[inline]
    pub fn take(&mut self) -> Option<wgpu::CommandEncoder> {
        self.encoder.take()
    }
}
