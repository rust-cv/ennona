[[location(0)]]
var<in> position: vec2<f32>;
[[location(1)]]
var<in> color: vec3<f32>;
[[location(0)]]
var<out> f_color: vec3<f32>;
[[builtin(position)]]
var<out> out_position: vec4<f32>;

[[stage(vertex)]]
fn vs_main() {
    f_color = color;
    out_position = vec4<f32>(position, 0.5, 1.0);
}

[[location(0)]]
var<in> f_color: vec3<f32>;
[[location(0)]]
var<out> out_color: vec4<f32>;

[[stage(fragment)]]
fn fs_main() {
    out_color = vec4<f32>(f_color, 1.0);
}