// Fragment simple
#version 320 es
#ifdef GL_ES
precision mediump float;
#endif

uniform vec3 lightDir;
uniform vec3 cameraPosition;
uniform sampler2D utexture;

in VertexData {
    vec2 texCoord;
    vec3 normal;
    vec3 worldPosition;
} VertexOut;

out vec4 fragColor;

void main() {
    vec3 N = normalize(VertexOut.normal);
    vec3 V = normalize(VertexOut.worldPosition - cameraPosition);
    vec3 L = normalize(VertexOut.worldPosition - lightDir);

    // Rim lighting factor
    float rimFactor = 1.0 - max(dot(N, V), 0.0);
    rimFactor = pow(rimFactor, 2.0);

    // diffuse toon step
    float diffuse = max(dot(N, L), 0.0);
    float toonStep = smoothstep(0.2, 0.8, diffuse);

    vec4 texColor = texture(utexture, VertexOut.texCoord);

    // combine lighting
    vec3 ambient = 0.1 * texColor.rgb;
    vec3 lighting = ambient + toonStep * texColor.rgb + rimFactor * vec3(0.3);
    fragColor = vec4(lighting, texColor.a);
}
