//! Elite WebGPU renderer rendering methods

use super::TextAreaData;
use super::elite_init::EliteWebGPURenderer;
use crate::config::Config;
use crate::error::CodeSkewError;
use crate::layout::PositionedLine;
use glyphon::*;

impl EliteWebGPURenderer {
    /// Render positioned lines to buffer
    pub fn render(
        &mut self,
        layout: &[PositionedLine],
        _config: &Config,
    ) -> Result<Vec<u8>, CodeSkewError> {
        // Update time
        let now = std::time::Instant::now();
        self.time_delta = now.duration_since(self.last_frame_time).as_secs_f32();
        self.time_elapsed += self.time_delta;
        self.frame_count += 1.0;
        self.last_frame_time = now;

        // Update uniforms with supersampled resolution for rendering
        self.uniforms.update(self.time_elapsed, self.time_delta);
        self.uniforms
            .update_resolution(self.supersampled_width as f32, self.supersampled_height as f32);

        self.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.uniforms]),
        );

        // Prepare text areas
        self.prepare_text_areas(layout)?;

        // Render composite
        self.render_composite_internal(layout)?;

        // Submit commands
        if let Some(command_buffer) = self.command_state.take() {
            self.queue.submit(std::iter::once(command_buffer.finish()));
        }

        // Read back result
        pollster::block_on(self.read_back_buffer())
    }

    /// Prepare text areas for rendering
    fn prepare_text_areas(&mut self, layout: &[PositionedLine]) -> Result<(), CodeSkewError> {
        self.text_state.areas_data.clear();
        self.string_builder.clear();

        for (i, line) in layout.iter().enumerate() {
            // Build rich text segments with individual colors
            let mut rich_text_segments = Vec::new();
            let mut current_segment = String::new();
            let mut current_color = None;

            for styled_char in &line.chars {
                let char_color = Color::rgb(styled_char.color.r, styled_char.color.g, styled_char.color.b);
                
                // If color changed, push current segment and start new one
                if current_color.is_some() && current_color != Some(char_color) {
                    if !current_segment.is_empty() {
                        let attrs = Attrs::new().color(current_color.unwrap());
                        rich_text_segments.push((current_segment.clone(), attrs));
                        current_segment.clear();
                    }
                }
                
                current_segment.push(styled_char.char);
                current_color = Some(char_color);
            }

            // Push final segment
            if !current_segment.is_empty() {
                let attrs = Attrs::new().color(current_color.unwrap_or(Color::rgb(255, 255, 255)));
                rich_text_segments.push((current_segment, attrs));
            }

            // Get or create buffer
            if i >= self.text_state.buffers.len() {
                self.text_state
                    .ensure_capacity(i + 1, &mut self.font_system, self.supersampled_width, self.supersampled_height, self.supersampling_factor);
            }

            let buffer = &mut self.text_state.buffers[i];
            
            // Use set_rich_text instead of set_text for per-segment coloring
            // Convert String to &str for the API
            let rich_text_refs: Vec<(&str, Attrs)> = rich_text_segments
                .iter()
                .map(|(text, attrs)| (text.as_str(), attrs.clone()))
                .collect();
            
            buffer.set_rich_text(
                &mut self.font_system,
                rich_text_refs,
                &Attrs::new(),
                Shaping::Advanced,
                None,
            );

            // Create text area data - scale by supersampling factor
            let area_data = TextAreaData {
                buffer_index: i,
                left: line.x * self.supersampling_factor,
                top: line.y * self.supersampling_factor,
                scale: self.supersampling_factor,
                bounds: TextBounds::default(),
                default_color: Color::rgb(255, 255, 255), // Fallback color
            };

            self.text_state.areas_data.push(area_data);
        }

        Ok(())
    }

    /// Render composite scene with 3x supersampling
    fn render_composite_internal(
        &mut self,
        positioned_lines: &[PositionedLine],
    ) -> Result<(), CodeSkewError> {
        // Create command encoder
        let encoder = self.command_state.get_or_create(&self.device);
        
        // First pass: Render to supersampled texture at 3x resolution
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Supersampled Clear Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.supersampled_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.composite_pipeline);
            render_pass.set_bind_group(0, &self.composite_bind_group, &[]);
            // Set viewport to supersampled dimensions
            render_pass.set_viewport(0.0, 0.0, self.supersampled_width as f32, self.supersampled_height as f32, 0.0, 1.0);
            render_pass.draw(0..3, 0..1);
            drop(render_pass);

            // Render text on top if we have positioned lines
            if !positioned_lines.is_empty() {
                let text_areas = self.text_state.prepare_areas();

                // Prepare text for rendering
                self.text_renderer
                    .prepare(
                        &self.device,
                        &self.queue,
                        &mut self.font_system,
                        &mut self.text_atlas,
                        &self.viewport,
                        text_areas,
                        &mut self.cache,
                    )
                    .map_err(|e| {
                        CodeSkewError::RenderingError(format!("Text preparation failed: {e:?}"))
                    })?;

                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Supersampled Text Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.supersampled_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                // Set viewport to supersampled dimensions for text rendering
                render_pass.set_viewport(0.0, 0.0, self.supersampled_width as f32, self.supersampled_height as f32, 0.0, 1.0);
                
                self.text_renderer
                    .render(&self.text_atlas, &self.viewport, &mut render_pass)
                    .map_err(|e| {
                        CodeSkewError::RenderingError(format!("Text rendering failed: {e:?}"))
                    })?;
            }
            
            // Second pass: Downsample from supersampled texture to composite texture
            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Downsample Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.composite_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                render_pass.set_pipeline(&self.downsample_pipeline);
                render_pass.set_bind_group(0, &self.downsample_bind_group, &[]);
                // Set viewport to final output dimensions
                render_pass.set_viewport(0.0, 0.0, self.width as f32, self.height as f32, 0.0, 1.0);
                render_pass.draw(0..3, 0..1);  // Fullscreen triangle
            }

            // Copy composite texture to buffer
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.composite_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &self.copy_buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(self.padded_bytes_per_row),
                        rows_per_image: Some(self.height),
                    },
                },
                wgpu::Extent3d {
                    width: self.width,
                    height: self.height,
                    depth_or_array_layers: 1,
                },
            );
        }

        Ok(())
    }

    /// Read back buffer data
    async fn read_back_buffer(&mut self) -> Result<Vec<u8>, CodeSkewError> {
        // Read back results
        {
            let buffer_slice = self.copy_buffer.slice(..);
            let (sender, receiver) = futures::channel::oneshot::channel();

            buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                sender.send(result).unwrap();
            });

            self.device.poll(wgpu::PollType::Wait).unwrap();

            match futures::executor::block_on(receiver) {
                Ok(Ok(())) => {
                    let padded_data = buffer_slice.get_mapped_range();

                    // Copy data, removing padding
                    for (row_idx, row_data) in self
                        .result_data
                        .chunks_exact_mut(self.unpadded_bytes_per_row as usize)
                        .enumerate()
                    {
                        let padded_start = row_idx * self.padded_bytes_per_row as usize;
                        let padded_end = padded_start + self.unpadded_bytes_per_row as usize;
                        if padded_end <= padded_data.len() {
                            row_data.copy_from_slice(&padded_data[padded_start..padded_end]);
                        }
                    }

                    drop(padded_data);
                    self.copy_buffer.unmap();

                    Ok(self.result_data.clone())
                }
                Ok(Err(e)) => Err(CodeSkewError::RenderingError(format!(
                    "Buffer mapping failed: {e:?}"
                ))),
                Err(_) => Err(CodeSkewError::RenderingError(
                    "Buffer mapping was cancelled".to_string(),
                )),
            }
        }
    }

    /// Update animation time
    pub fn update_time(&mut self, delta_time: f32) {
        self.time_elapsed += delta_time;
        self.time_delta = delta_time;
        self.frame_count += 1.0;

        self.uniforms.update(self.time_elapsed, self.time_delta);
        self.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.uniforms]),
        );
    }

    /// Set mouse position
    pub fn set_mouse(&mut self, x: f32, y: f32, clicked_x: f32, clicked_y: f32) {
        self.uniforms.update_mouse(x, y, clicked_x, clicked_y);
        self.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.uniforms]),
        );
    }
}
