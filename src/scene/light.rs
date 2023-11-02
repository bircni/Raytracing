// Import necessary Rust crates and modules
use nalgebra::Point3;   // 3D point data type from the nalgebra crate
use serde::Deserialize; // Serialization and deserialization support

// Import the custom Color data structure from the current crate
use crate::Color;

// Define a custom struct 'Light' for representing a light source
#[derive(Debug, Clone, PartialEq)]
pub struct Light {
    position: Point3<f32>, // Position of the light source in 3D space
    color: Color,         // Color of the light source
    intensity: f32,       // Intensity of the light source
}

// Implement a custom deserializer for 'Light'
impl<'de> Deserialize<'de> for Light {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Define a nested module 'yaml' for deserializing YAML data
        mod yaml {
            use nalgebra::Point3; // 3D point data type from nalgebra
            use serde::Deserialize;

            // Import the custom Color data structure from the parent module
            use crate::Color;

            // Define a struct 'Light' for deserializing light data from YAML
            #[derive(Deserialize)]
            pub struct Light {
                // Deserialize position as a Point3 using a custom deserializer
                #[serde(with = "super::super::yaml::point3_xyz")]
                pub position: Point3<f32>,

                // Deserialize color as Color using a custom deserializer
                #[serde(with = "super::super::yaml::color_rgb")]
                pub ke: Color,
            }
        }

        // Deserialize the light data using the 'yaml' module's 'Light' struct
        yaml::Light::deserialize(deserializer).map(|yaml_light| Light {
            // Extract and store the position from the YAML data
            position: yaml_light.position,

            // Normalize and store the color from the YAML data
            color: yaml_light.ke.normalize(),

            // Calculate and store the intensity of the light source
            intensity: yaml_light.ke.norm(),
        })
    }
}
