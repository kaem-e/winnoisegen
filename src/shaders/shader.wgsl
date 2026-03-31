const pos = array<vec2f, 4>(
	vec2f(-0.5,  0.5),  // top left
	vec2f( 0.5,  0.5),  // top right
	vec2f(-0.5, -0.5),  // bottom left
	vec2f( 0.5, -0.5)   // bottom right
);

const color = array<vec3f, 4>(
	vec3f(1.00, 0.53, 0.36),
	vec3f(0.91, 0.02, 0.40),
	vec3f(0.06, 0.81, 0.86),
	vec3f(0.43, 0.38, 1.00),
);

struct VertexOutput {
	@builtin(position) position: vec4f,
	@location(0) color: vec3f,
	@location(1) local_pos: vec2f,
}

@vertex
fn vert_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
	var out: VertexOutput;
	if vertex_index < 3 {
		out.position = vec4f(pos[vertex_index], 0.0, 1.0);
		out.color = color[vertex_index];
		out.local_pos = pos[vertex_index];
	} else {
		let vertex_index = vertex_index - 3 + 1;
		out.position = vec4f(pos[vertex_index], 0.0, 1.0);
		out.color = color[vertex_index];
		out.local_pos = pos[vertex_index];
	}
	return out;
}

@fragment
fn frag_main(input: VertexOutput) -> @location(0) vec4f {
	// Distance from the center (0,0)
	let dist = length(input.local_pos);

	// Fade out as distance increases.
	// 0.5 is the distance to the furthest vertex in your 'pos' array.
	let alpha = smoothstep(0.5, 0.0, dist);

	return vec4f(input.color, alpha);
	// return vec4f(input.color, 1.0);
}
