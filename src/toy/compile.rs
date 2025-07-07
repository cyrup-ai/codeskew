use crate::toy::{WgpuToyRenderer, pp::SourceMap, ComputePipeline};
use lazy_regex::regex;

impl WgpuToyRenderer {
    /// Compile shader source into compute pipeline - the missing piece!
    pub fn compile(&mut self, source: SourceMap) {
        let now = instant::Instant::now();
        
        // Generate prelude with binding declarations
        let prelude = self.prelude();
        let wgsl = format!("{}{}", prelude, source.source);
        
        // Parse entry points from the complete WGSL
        let re_entry_point = regex!(r"(?s)@compute.*?@workgroup_size\((.*?)\).*?fn\s+(\w+)");
        let entry_points: Vec<(String, [u32; 3])> = re_entry_point
            .captures_iter(&wgsl)
            .map(|cap| {
                let mut sizes = cap[1].split(',').map(|s| s.trim().parse().unwrap_or(1));
                let workgroup_size: [u32; 3] = std::array::from_fn(|_| sizes.next().unwrap_or(1));
                (cap[2].to_owned(), workgroup_size)
            })
            .collect();

        // Create compute pipelines from entry points
        self.compute_pipelines.clear();
        for (entry_point, workgroup_size) in entry_points {
            let compute_shader = self.wgpu.device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(&format!("Compute Shader - {}", entry_point)),
                source: wgpu::ShaderSource::Wgsl(wgsl.clone().into()),
            });

            let compute_pipeline = self.wgpu.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(&format!("Compute Pipeline - {}", entry_point)),
                layout: Some(&self.compute_pipeline_layout),
                module: &compute_shader,
                entry_point: Some(&entry_point),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

            self.compute_pipelines.push(ComputePipeline {
                name: entry_point,
                pipeline: compute_pipeline,
                workgroup_size,
                workgroup_count: None,
                dispatch_once: false,
                dispatch_count: 1,
            });
        }

        println!("🔧 DEBUG: Compiled {} compute pipelines in {:.3}s", 
                 self.compute_pipelines.len(), 
                 now.elapsed().as_secs_f32());
        
        self.source = source;
    }
    
    /// Generate prelude with all binding declarations
    fn prelude(&self) -> String {
        let mut s = String::new();
        
        // Type aliases for convenience
        for (a, t) in [("int", "i32"), ("uint", "u32"), ("float", "f32")] {
            s.push_str(&format!("alias {a} = {t};\n"));
        }
        for (a, t) in [("int", "i32"), ("uint", "u32"), ("float", "f32"), ("bool", "bool")] {
            for n in 2..5 {
                s.push_str(&format!("alias {a}{n} = vec{n}<{t}>;\n"));
            }
        }
        for n in 2..5 {
            for m in 2..5 {
                s.push_str(&format!("alias float{n}x{m} = mat{n}x{m}<f32>;\n"));
            }
        }
        
        // Standard structs
        s.push_str(r#"
struct Time { frame: uint, elapsed: float, delta: float }
struct Mouse { pos: uint2, click: int }
struct DispatchInfo { id: uint }
"#);
        
        // Custom struct (simplified for now)
        s.push_str("struct Custom { _dummy: float };\n");
        
        // Data struct (simplified for now)  
        s.push_str("struct Data { _dummy: array<u32,1> };\n");
        
        // All binding declarations
        s.push_str(&self.bindings.to_wgsl());
        
        // Helper functions
        s.push_str(r#"
fn keyDown(keycode: uint) -> bool {
    return ((_keyboard[keycode / 128u][(keycode % 128u) / 32u] >> (keycode % 32u)) & 1u) == 1u;
}
"#);
        
        s
    }
}