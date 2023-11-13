#version 430 core

out vec4 FragColor;

in vec3 FragPos;
in vec3 FragNormal;
in vec3 FragColorIn;

struct Light {
    vec3 position;
    vec3 color;
    float intensity;
};

layout(std430, binding = 0)buffer LightsBuffer {
    Light lights[];
};

const float AMBIENT_INTENSITY = 0.1;

void main() {
    vec3 color = AMBIENT_INTENSITY * FragColorIn;
    
    for(uint i = 0; i < lights.length(); ++ i) {
        float distance = length(lights[i].position - FragPos);
        
        // diffuse shading
        vec3 lightDir = normalize(lights[i].position - FragPos);
        float diff = max(dot(FragNormal, lightDir), 0.0);
        color += diff * lights[i].intensity * lights[i].color * FragColorIn / pow(distance, 2);
    }
    
    FragColor = vec4(color, 1.0);
}