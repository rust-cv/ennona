struct VertexOutput {
    @builtin(position) out_position: vec4<f32>,
    @location(0) f_color: vec3<f32>,
};

@stage(vertex)
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    out.f_color = color;
    out.out_position = vec4<f32>(position, 1.0);
    return out;
}


@stage(fragment)
fn fs_main(
    in: VertexOutput
) -> @location(0) vec4<f32> {
    return vec4<f32>(in.f_color, 1.0);
}