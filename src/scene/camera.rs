use nalgebra::{Point3, Vector3};
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq)]
pub struct Camera {
    pub position: Point3<f32>,
    pub look_at: Point3<f32>,
    pub up: Vector3<f32>,
    pub fov: f32,
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
            fov: yaml_camera.field_of_view,
        })
    }
}
