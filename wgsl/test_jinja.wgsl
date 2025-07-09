// Test MiniJinja templating in WGSL
// This shader demonstrates dynamic code generation using Jinja2 templates

{% set num_octaves = 5 %}
{% set grid_size = 32 %}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let resolution = vec2<f32>({{ SCREEN_WIDTH }}, {{ SCREEN_HEIGHT }});
    let pos = vec2<f32>(global_id.xy);
    
    if (pos.x >= resolution.x || pos.y >= resolution.y) {
        return;
    }
    
    let uv = pos / resolution;
    let t = time.elapsed;
    
    // Generate fractal noise with {{ num_octaves }} octaves
    var noise = 0.0;
    var amplitude = 1.0;
    var frequency = 1.0;
    
    {% for i in range(0, num_octaves) %}
    // Octave {{ i }}
    noise += amplitude * sin(frequency * pos.x * 0.01 + t) * cos(frequency * pos.y * 0.01 + t * 0.7);
    amplitude *= 0.5;
    frequency *= 2.0;
    {% endfor %}
    
    // Create a grid pattern
    let grid = vec2<f32>(
        {% if grid_size > 16 %}
        floor(uv.x * {{ grid_size }}) / {{ grid_size }},
        floor(uv.y * {{ grid_size }}) / {{ grid_size }}
        {% else %}
        uv.x,
        uv.y
        {% endif %}
    );
    
    // Mathematical constants from template context
    let phase = t * {{ PI }} * 0.5;
    let wave = sin(phase + grid.x * {{ TAU }}) * cos(phase + grid.y * {{ TAU }});
    
    // Final color
    let color = vec3<f32>(
        0.5 + 0.5 * noise,
        0.5 + 0.5 * wave,
        0.5 + 0.5 * sin(t + uv.x * {{ E }})
    );
    
    textureStore(screen, vec2<i32>(global_id.xy), vec4<f32>(color, 1.0));
}