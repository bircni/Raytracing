// Import necessary Rust crates and modules
use nalgebra::{Point3, Vector3}; // 3D point and vector data types from nalgebra
use serde::Deserialize;          // Serialization and deserialization support

// Define a custom struct 'Camera' for representing a camera in a 3D scene
#[derive(Debug, Clone, PartialEq)]
pub struct Camera {
    position: Point3<f32>,   // Position of the camera in 3D space
    direction: Vector3<f32>, // Direction the camera is pointing
    up: Vector3<f32>,        // Up vector for camera orientation
    fov: f32,                // Field of view (FOV) for the camera
}

// Define a module 'yaml' for deserializing YAML data
mod yaml {
    use nalgebra::{Point3, Vector3}; // 3D point and vector data types from nalgebra
    use serde::Deserialize;          // Serialization and deserialization support

    // Define a struct 'Camera' for deserializing camera data from YAML
    #[derive(Deserialize)]
    pub struct Camera {
        // Deserialize position as a Point3 using a custom deserializer
        #[serde(with = "super::super::yaml::point3_xyz")]
        pub position: Point3<f32>,

        // Deserialize look_at point as a Point3 using a custom deserializer
        #[serde(with = "super::super::yaml::point3_xyz")]
        pub look_at: Point3<f32>,

        // Deserialize up vector as a Vector3 using a custom deserializer
        #[serde(with = "super::super::yaml::vector3_xyz")]
        pub up_vec: Vector3<f32>,

        // Deserialize field of view as a floating-point number
        pub field_of_view: f32,
    }
}

// Implement a custom deserializer for the 'Camera' struct
impl<'de> Deserialize<'de> for Camera {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Deserialize the camera data using the 'yaml' module's 'Camera' struct
        yaml::Camera::deserialize(deserializer).map(|yaml_camera| Camera {
            // Extract and store the camera's position
            position: yaml_camera.position,

            // Calculate and store the camera's direction as a normalized vector
            direction: (yaml_camera.look_at - yaml_camera.position).normalize(),

            // Extract and store the camera's up vector
            up: yaml_camera.up_vec,

            // Extract and store the camera's field of view
            fov: yaml_camera.field_of_view,
        })
    }
}
