use nalgebra::{Point3, Rotation3, Vector3};

use crate::raytracer::Ray;

#[derive(Debug, Clone, PartialEq)]
pub struct Camera {
    pub position: Point3<f32>,
    pub look_at: Point3<f32>,
    pub up: Vector3<f32>,
    pub fov: f32,
    pub resolution: (u32, u32),
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Point3::new(1.0, 1.0, 1.0),
            look_at: Point3::origin(),
            up: Vector3::y(),
            fov: 60.0_f32.to_radians(),
            resolution: (1920, 1080),
        }
    }
}

impl Camera {
    /// Returns a ray from the given pixel coordinates.
    /// x and y are in the range -1..1 and represent
    /// the relative position of the pixel in the image.
    /// (0, 0) is the center of the image.
    pub fn ray(&self, x: f32, y: f32) -> Ray {
        // direction in coordinate system of camera
        let direction = Vector3::new(x, -y, -1.0 / (self.fov / 2.0).tan());

        // rotate direction to world coordinate system
        let rotation = Rotation3::look_at_rh(&(self.look_at - self.position), &self.up);
        let direction = rotation.inverse_transform_vector(&direction);

        Ray {
            origin: self.position,
            direction: direction.normalize(),
        }
    }
}

mod yaml {
    use nalgebra::{Point3, Vector3};
    use serde::{Deserialize, Serialize};

    use super::Camera;

    #[derive(Serialize, Deserialize)]
    pub struct CameraDef {
        #[serde(with = "super::super::yaml::point")]
        pub position: Point3<f32>,
        #[serde(rename = "lookAt")]
        #[serde(with = "super::super::yaml::point")]
        pub look_at: Point3<f32>,
        #[serde(rename = "upVec")]
        #[serde(with = "super::super::yaml::vector")]
        pub up_vec: Vector3<f32>,
        #[serde(rename = "fieldOfView")]
        pub field_of_view: f32,
        pub width: u32,
        pub height: u32,
    }

    impl<'de> Deserialize<'de> for Camera {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            CameraDef::deserialize(deserializer).map(|yaml_camera| Camera {
                position: yaml_camera.position,
                look_at: yaml_camera.look_at,
                up: yaml_camera.up_vec,
                fov: yaml_camera.field_of_view.to_radians(),
                resolution: (yaml_camera.width, yaml_camera.height),
            })
        }
    }

    impl Serialize for Camera {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            CameraDef {
                position: self.position,
                look_at: self.look_at,
                up_vec: self.up,
                field_of_view: self.fov.to_degrees(),
                width: self.resolution.0,
                height: self.resolution.1,
            }
            .serialize(serializer)
        }
    }
}
