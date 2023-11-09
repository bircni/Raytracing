#version 430 core

layout(location = 0)in vec3 Position;
layout(location = 1)in vec3 Normal;
layout(location = 2)in vec3 Color;

out vec3 FragPos;
out vec3 FragNormal;
out vec3 FragColorIn;

uniform mat4 view;

void main() {
    FragPos = Position;
    FragNormal = Normal;
    FragColorIn = Color;
    
    gl_Position = view * vec4(Position, 1.0);
}
