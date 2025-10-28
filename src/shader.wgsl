// shader.wgsl
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(1) color: vec3<f32>,
};

@vertex
fn vs_main(@location(0) position: vec2<f32>, @location(1) color: vec3<f32>) -> VertexOutput {
    var out: VertexOutput;
    // coordinate in pixel -> [-1,1] nello spazio clip
    let screen_size = vec2<f32>(800.0, 600.0);
    out.position = vec4<f32>((position / screen_size) * 2.0 - vec2<f32>(1.0, 1.0), 0.0, 1.0);
    out.color = color;
    return out;
}

@fragment
fn fs_main(@location(1) color: vec3<f32>) -> @location(0) vec4<f32> {
    return vec4<f32>(color, 1.0);
}
