// Fragment shader: unlit
#version 320 es
precision mediump float;

uniform sampler2D u_texture;
in vec2 v_uv;
out vec4 frag_color;

void main() {
    frag_color = texture(u_texture, v_uv);
}
