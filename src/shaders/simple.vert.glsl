// Vertex simple
#version 320 es

in vec3 position;
in vec3 normal;
in vec2 tex_coord;

uniform mat4 modelMatrix;
uniform mat4 viewMatrix;
uniform mat4 projectionMatrix;
uniform mat3 normalMatrix;

out VertexData {
    vec2 texCoord;
    vec3 normal;
    vec3 worldPosition;
} VertexOut;

void main() {
    vec4 worldPosition = modelMatrix * vec4(position, 1.0);
    VertexOut.worldPosition = worldPosition.xyz;
    VertexOut.texCoord = tex_coord;
    VertexOut.normal = normalize(normalMatrix * normal);
    gl_Position = projectionMatrix * viewMatrix * worldPosition;
}
