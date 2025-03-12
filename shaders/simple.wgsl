// Vertex shader

struct VertexOutput {
	// Like gl_position
	// Gives us the pixel that we are drawing for 
	// y = 0 is the top of the screen
	@builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vs_main(
	@builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
	var out: VertexOutput;
	// f32() is a cast 
	// var is mutable but needs to have type clarified
	// let is immutable but has infered typd
	let x = f32(1 - i32(in_vertex_index)) * 0.5;
	let y = f32(i32(in_vertex_index & 1u) * 2 - 1) * 0.5;
	out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
	return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.3, 0.2, 0.1, 1.0);
}
