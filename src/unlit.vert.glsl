#version 320 es
// Inputs from your vertex buffer
layout(location = 0) in vec3 a_position;
layout(location = 1) in vec2 a_uv;

// Uniforms
uniform mat4 u_model;
uniform mat4 u_view;
uniform mat4 u_proj;

// Outputs to fragment shader
out vec2 v_uv;

void main() {
    // Transform to clip space
    gl_Position = u_proj * u_view * u_model * vec4(a_position, 1.0);

    // Pass UV to fragment shader
    v_uv = a_uv;
}
