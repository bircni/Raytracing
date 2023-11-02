// Import necessary Rust crates and modules
use anyhow::Context; // Error handling and context for `anyhow` crate
use serde::Deserialize; // Serialization and deserialization support
use serde_yaml::Value; // YAML value for deserialization

// Import modules for custom data structures
use self::{camera::Camera, light::Light, object::Object, settings::Settings};

mod camera; // Module for Camera
mod light;  // Module for Light
pub mod object; // Module for Object
pub mod settings; // Module for Settings
mod triangle; // Module for Triangle

// Define a custom struct 'Scene' for representing a 3D scene
#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    objects: Vec<Object>, // A collection of objects in the scene
    lights: Vec<Light>,   // A collection of lights in the scene
    camera: Camera,       // The camera used to view the scene
    settings: Settings,   // Additional settings for the scene
}

// Implement a custom deserializer for 'Scene'
impl<'de> Deserialize<'de> for Scene {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Deserialize the YAML configuration into a 'Value'
        let config = Value::deserialize(deserializer)?;

        // Deserialize objects, lights, camera, and settings from the 'Value'
        let objects = config
            .get("models")
            .ok_or(serde::de::Error::missing_field("models"))?
            .as_sequence()
            .ok_or(serde::de::Error::custom("models is not a sequence"))?
            .into_iter()
            .map(|s| serde_yaml::from_value::<Object>(s.clone()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(serde::de::Error::custom)?;

        let lights = config
            .get("point_lights")
            .ok_or(serde::de::Error::missing_field("point_lights"))?
            .as_sequence()
            .ok_or(serde::de::Error::custom("point_lights is not a sequence"))?
            .into_iter()
            .map(|s| serde_yaml::from_value::<Light>(s.clone()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(serde::de::Error::custom)?;

        let camera = serde_yaml::from_value::<Camera>(
            config
                .get("camera")
                .ok_or(serde::de::Error::missing_field("camera"))?
                .clone(),
        )
        .map_err(serde::de::Error::custom)?;

        let settings = serde_yaml::from_value::<Settings>(
            config
                .get("extra_args")
                .ok_or(serde::de::Error::missing_field("extra_args"))?
                .clone(),
        )
        .map_err(serde::de::Error::custom)?;

        // Create and return the 'Scene' instance
        Ok(Scene {
            objects,
            lights,
            camera,
            settings,
        })
    }
}

// Module for custom YAML deserialization functions
mod yaml {
    pub mod point3_xyz {
        // Custom deserialization for Point3 from XYZ data
        use nalgebra::Point3;
        use serde::Deserialize;

        #[derive(serde::Deserialize)]
        pub struct XYZ {
            x: f32,
            y: f32,
            z: f32,
        }

        pub fn deserialize<'de, D>(deserializer: D) -> Result<Point3<f32>, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let point = XYZ::deserialize(deserializer)?;
            Ok(Point3::new(point.x, point.y, point.z))
        }
    }

    pub mod vector3_xyz {
        // Custom deserialization for Vector3 from XYZ data
        use nalgebra::Vector3;
        use serde::Deserialize;

        #[derive(serde::Deserialize)]
        pub struct XYZ {
            x: f32,
            y: f32,
            z: f32,
        }

        pub fn deserialize<'de, D>(deserializer: D) -> Result<Vector3<f32>, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let point = XYZ::deserialize(deserializer)?;
            Ok(Vector3::new(point.x, point.y, point.z))
        }
    }

    pub mod color_rgb {
        // Custom deserialization for Color from RGB data
        use serde::Deserialize;
        use crate::Color;

        #[derive(serde::Deserialize)]
        pub struct RGB {
            r: f32,
            g: f32,
            b: f32,
        }

        pub fn deserialize<'de, D>(deserializer: D) -> Result<Color, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let color = RGB::deserialize(deserializer)?;
            Ok(Color::new(color.r, color.g, color.b))
        }
    }
}

impl Scene {
    // Define a method to load a scene from a file
    pub fn load<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<Scene> {
        // Read the file content to a string and handle errors
        let s = std::fs::read_to_string(path.as_ref()).context(format!(
            "Failed to read file from path: {}",
            path.as_ref().display()
        ))?;

        // Parse the YAML content into a 'Scene' and handle errors
        serde_yaml::from_str::<Scene>(s.as_str()).context("Failed to parse yaml config file")
    }
}

// Test function to verify scene loading
#[test]
fn test_load_scene() -> anyhow::Result<()> {
    // Load the scene from a specified YAML configuration file
    let scene = Scene::load("./res/config.yaml").context("Failed to load scene")?;

    // Verify that the scene contains the expected number of objects and lights
    assert_eq!(scene.objects.len(), 2);
    assert_eq!(scene.lights.len(), 2);

    Ok(())
}
