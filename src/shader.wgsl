// shader.wgsl

// 1. Definiamo la struttura dei nostri dati "Uniform"
// Deve corrispondere a una struct che creeremo in Rust.
struct Globals {
    screen_size: vec2<f32>,
};

// 2. Dichiariamo l'uniform.
// @group(0) @binding(0) dice a WGPU di collegare qui
// il buffer che specificheremo in Rust.
@group(0) @binding(0)
var<uniform> u_globals: Globals;


struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(1) color: vec3<f32>,
};

@vertex
fn vs_main(@location(0) position: vec2<f32>, @location(1) color: vec3<f32>) -> VertexOutput {
    var out: VertexOutput;
    
    // 3. Rimuoviamo il magic number!
    // let screen_size = vec2<f32>(800.0, 600.0); // <-- RIMOSSO

    // 4. Usiamo il nostro nuovo uniform!
    let screen_size = u_globals.screen_size;
    
    let clip_pos = vec2<f32>(
        (position.x / screen_size.x) * 2.0 - 1.0,
        (position.y / screen_size.y) * -2.0 + 1.0
    );

    out.position = vec4<f32>(clip_pos, 0.0, 1.0);
    out.color = color;
    return out;
}

@fragment
fn fs_main(@location(1) color: vec3<f32>) -> @location(0) vec4<f32> {
    return vec4<f32>(color, 1.0);
}