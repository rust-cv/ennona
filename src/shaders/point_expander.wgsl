struct Uniforms {
    projection: mat4x4<f32>;
    pixel_size: f32;
};

[[group(0), binding(0)]]
var<uniform> uniforms: Uniforms;

struct Vertex {
    position: vec3<f32>;
    color: vec3<f32>;
};

struct SourceVertices {
    vertices: [[stride(32)]] array<Vertex>;
};

[[group(1), binding(0)]]
var<storage, read> source: SourceVertices;

struct SinkVertices {
    vertices: [[stride(32)]] array<Vertex>;
};

[[group(1), binding(1)]]
var<storage, read_write> sink: SinkVertices;

[[stage(compute), workgroup_size(64)]]
fn main([[builtin(global_invocation_id)]] id: vec3<u32>) {
    let ix = id.x;
    let original_vertex = source.vertices[ix];
    // Compute the actual final point position.
    let center_position = uniforms.projection * vec4<f32>(original_vertex.position, 1.0);
    let center_position = vec3<f32>(center_position.x, center_position.y, center_position.z) / center_position.w;
    for (var i = 0u; i < 3u; i = i + 1u) {
        // Set the color the same for all three.
        sink.vertices[ix * 3u + i].color = original_vertex.color;
    }
    // each of the verticies will be unique.
    sink.vertices[ix * 3u + 0u].position = center_position + vec3<f32>(vec2<f32>(0.0, -1.0) * uniforms.pixel_size, 0.0);
    sink.vertices[ix * 3u + 1u].position = center_position + vec3<f32>(vec2<f32>(-0.86602540378, 0.5) * uniforms.pixel_size, 0.0);
    sink.vertices[ix * 3u + 2u].position = center_position + vec3<f32>(vec2<f32>(0.86602540378, 0.5) * uniforms.pixel_size, 0.0);
}   
