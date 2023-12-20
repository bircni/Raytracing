use anyhow::Context;
use serde::{Deserialize, Serialize};

pub use self::{camera::Camera, light::Light, object::Object, settings::Settings};

mod camera;
mod light;
mod object;
mod settings;
mod triangle;
mod yaml;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    #[serde(rename = "models")]
    pub objects: Vec<Object>,
    #[serde(rename = "point_lights")]
    pub lights: Vec<Light>,
    pub camera: Camera,
    #[serde(rename = "extra_args")]
    pub settings: Settings,
}

impl Scene {
    pub fn load<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<Scene> {
        let s = std::fs::read_to_string(path.as_ref()).context(format!(
            "Failed to read file from path: {}",
            path.as_ref().display()
        ))?;

        serde_yaml::from_str::<Scene>(s.as_str()).context("Failed to parse yaml config file")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Color;
    use nalgebra::{Point3, Similarity3, Vector3};

    #[test]
    fn test_load_scene() -> anyhow::Result<()> {
        // test loading of scene
        let scene = Scene::load("./res/test_config.yaml").context("Failed to load scene")?;

        // test loading of objects
        assert_eq!(scene.objects.len(), 1);
        assert_eq!(
            scene.objects[0].transform,
            Similarity3::from_parts(
                nalgebra::Translation3::from(Vector3::new(0.7, -0.1, -0.5)),
                nalgebra::UnitQuaternion::from_euler_angles(
                    17.761_688 * std::f32::consts::PI * 2.0 / 360.0,
                    -12.605_077 * std::f32::consts::PI * 2.0 / 360.0,
                    -6.561_449_5e-7 * std::f32::consts::PI * 2.0 / 360.0
                ),
                1.0
            )
        );

        // test loading of lights
        assert_eq!(scene.lights.len(), 2);
        assert_eq!(scene.lights[0].position, Point3::new(5.0, 2.0, 2.0));
        assert_eq!(
            scene.lights[0].color,
            Color::new(5.332_843, 11.98801, 12.322_679).normalize()
        );
        assert_eq!(scene.lights[1].position, Point3::new(-5.0, 5.0, -2.0));
        assert_eq!(
            scene.lights[1].color,
            Color::new(25.186_932, 4.616_624, 15.630_268).normalize()
        );

        // test loading of camera
        assert_eq!(scene.camera.position, Point3::new(-2.0, 1.5, 5.0));
        assert_eq!(scene.camera.look_at, Point3::new(0.0, 0.0, 0.0));
        assert_eq!(scene.camera.up, Vector3::new(0.0, 1.0, 0.0));
        assert!(f32::abs(scene.camera.fov - 80.21409_f32.to_radians()) < f32::EPSILON);

        // test loading of settings
        assert_eq!(scene.settings.max_bounces, 3);
        assert_eq!(scene.settings.samples, 64);
        assert_eq!(
            scene.settings.background_color,
            Color::new(0.672_961_65, 0.836_999_1, 0.664_332_1)
        );
        assert_eq!(
            scene.settings.ambient_color,
            Color::new(0.174_787_74, 0.170_237_97, 0.085_217_56).normalize()
        );

        Ok(())
    }
}
