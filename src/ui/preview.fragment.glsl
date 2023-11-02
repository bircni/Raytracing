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
    vec3 ambient = AMBIENT_INTENSITY * FragColorIn;
    vec3 diffuse = vec3(0.0);
    
    for(uint i = 0; i < lights.length(); i ++ ) {
        vec3 lightDir = normalize(lights[i].position - FragPos);
        float diff = max(dot(FragNormal, lightDir), 0.0);
        float distance = length(lights[i].position - FragPos);
        diffuse += diff * lights[i].color * lights[i].intensity / (distance * distance);
    }
    
    FragColor = vec4(ambient + diffuse, 1.0);
}