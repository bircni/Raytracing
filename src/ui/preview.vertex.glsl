#version 430 core

layout(location = 0)in vec3 Position;
layout(location = 1)in vec3 Normal;
layout(location = 2)in vec3 Color;
layout(location = 3)in uint TransformIndex;

out vec3 FragPos;
out vec3 FragNormal;
out vec3 FragColorIn;

uniform mat4 view;

layout(std430, binding = 1)buffer TransformBuffer {
    mat4 transforms[];
};

void main() {
    mat4 transform = transforms[TransformIndex];
    
    FragPos = vec3(transform * vec4(Position, 1.0));
    FragNormal = vec3(transform * vec4(Normal, 0.0));
    FragColorIn = Color;
    
    gl_Position = view * transform * vec4(Position, 1.0);
}
