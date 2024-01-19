use std::path::PathBuf;

use anyhow::Context;
use log::warn;
use serde::{de::DeserializeSeed, Deserialize, Serialize};

pub use self::{
    camera::Camera, light::Light, material::Material, object::Object, settings::Settings,
    skybox::Skybox,
};

mod camera;
mod light;
mod material;
mod object;
mod settings;
mod skybox;
mod triangle;
mod yaml;

#[derive(Debug, Clone, Serialize)]
pub struct Scene {
    #[serde(skip)]
    pub path: PathBuf,
    #[serde(rename = "models")]
    pub objects: Vec<Object>,
    #[serde(rename = "pointLights")]
    pub lights: Vec<Light>,
    pub camera: Camera,
    #[serde(rename = "extraArgs", default)]
    pub settings: Settings,
}

struct WithRelativePath<P: AsRef<std::path::Path>>(P);

impl<'de, P: AsRef<std::path::Path>> serde::de::DeserializeSeed<'de> for WithRelativePath<P> {
    type Value = Scene;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let map = <serde_yaml::Value as serde::Deserialize>::deserialize(deserializer)?;

        let objects = map
            .get("models")
            .ok_or_else(|| serde::de::Error::missing_field("models"))?
            .as_sequence()
            .ok_or_else(|| {
                serde::de::Error::invalid_type(serde::de::Unexpected::Map, &"a sequence")
            })?
            .iter()
            .map(|v| object::WithRelativePath(self.0.as_ref()).deserialize(v))
            .collect::<Result<Vec<Object>, serde_yaml::Error>>()
            .map_err(serde::de::Error::custom)?;

        let lights = map
            .get("pointLights")
            .ok_or_else(|| serde::de::Error::missing_field("pointLights"))?
            .as_sequence()
            .ok_or_else(|| {
                serde::de::Error::invalid_type(serde::de::Unexpected::Map, &"a sequence")
            })?
            .iter()
            .map(Light::deserialize)
            .collect::<Result<Vec<Light>, serde_yaml::Error>>()
            .map_err(serde::de::Error::custom)?;

        let camera = map
            .get("camera")
            .ok_or_else(|| serde::de::Error::missing_field("camera"))?;
        let camera = Camera::deserialize(camera).map_err(serde::de::Error::custom)?;

        let settings = map
            .get("extraArgs")
            .map(Settings::deserialize)
            .transpose()
            .map_err(|e| {
                warn!("Failed to deserialize extraArgs: {}", e);
                e
            })
            .unwrap_or_default()
            .unwrap_or_default();

        let scene = Scene {
            path: self.0.as_ref().to_path_buf(),
            objects,
            lights,
            camera,
            settings,
        };

        Ok(scene)
    }
}

impl Scene {
    pub fn load<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<Scene> {
        let s = std::fs::read_to_string(path.as_ref()).context(format!(
            "Failed to read file from path: {}",
            path.as_ref().display()
        ))?;

        WithRelativePath(path.as_ref())
            .deserialize(serde_yaml::Deserializer::from_str(&s))
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to deserialize scene from path: {}\n{}",
                    path.as_ref().display(),
                    e
                )
            })
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
        let scene =
            Scene::load("./res/e2e-test/test_config.yaml").context("Failed to load scene")?;

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
