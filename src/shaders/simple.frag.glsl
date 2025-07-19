// Fragment simple
#version 320 es
#ifdef GL_ES
precision mediump float;
#endif

uniform vec3 lightDir;
uniform sampler2D utexture;

in VertexData {
    vec2 texCoord;
    vec3 normal;
} VertexOut;

out vec4 fragColor;

void main() {
    vec3 N = normalize(VertexOut.normal);
    float NdotL = max(dot(N, normalize(lightDir)), 0.0);
    vec4 texColor = texture(utexture, VertexOut.texCoord);
    vec3 ambient = 0.1 * texColor.rgb;
    vec3 lighting = ambient + NdotL * texColor.rgb;
    // fragColor = vec4(lighting, texColor.a);
    fragColor = texColor;
}
