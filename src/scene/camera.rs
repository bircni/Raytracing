use nalgebra::{Point3, Rotation3, Unit, Vector3};
use serde::Deserialize;

use crate::raytracer::Ray;

#[derive(Debug, Clone, PartialEq)]
pub struct Camera {
    pub position: Point3<f32>,
    pub look_at: Point3<f32>,
    pub up: Vector3<f32>,
    pub fov: f32,
}

impl Camera {
    /// Returns a ray from the given pixel coordinates.
    /// x and y are in the range -1..1 and represent
    /// the relative position of the pixel in the image.
    /// (0, 0) is the center of the image.
    pub fn ray(&self, x: f32, y: f32) -> Ray {
        let origin = self.position;

        let y = (self.fov / 2.0 * y).atan();
        let x = (self.fov / 2.0 * x).atan();

        let center = (self.look_at - self.position).normalize();
        let direction = Rotation3::from_axis_angle(&Unit::new_normalize(self.up.cross(&center)), y)
            * Rotation3::from_axis_angle(&Unit::new_normalize(self.up), -x)
            * center;

        Ray {
            origin,
            direction: direction.normalize(),
        }
    }
}

mod yaml {
    use nalgebra::{Point3, Vector3};
    use serde::Deserialize;

    #[derive(Deserialize)]
    pub struct Camera {
        #[serde(with = "super::super::yaml::point3_xyz")]
        pub position: Point3<f32>,
        #[serde(with = "super::super::yaml::point3_xyz")]
        pub look_at: Point3<f32>,
        #[serde(with = "super::super::yaml::vector3_xyz")]
        pub up_vec: Vector3<f32>,
        pub field_of_view: f32,
    }
}

impl<'de> Deserialize<'de> for Camera {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        yaml::Camera::deserialize(deserializer).map(|yaml_camera| Camera {
            position: yaml_camera.position,
            look_at: yaml_camera.look_at,
            up: yaml_camera.up_vec,
            fov: yaml_camera.field_of_view.to_radians(),
        })
    }
}
