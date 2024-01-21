use std::path::PathBuf;

use anyhow::Context;
use log::warn;
use nalgebra::Vector3;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
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

pub type Color = Vector3<f32>;

#[derive(Debug, Serialize)]
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

impl Clone for Scene {
    fn clone(&self) -> Self {
        // Cloning a scene is necessary in some cases, but
        // it may be a very expensive operation, so we issue
        // a warning when in debug mode
        #[cfg(debug_assertions)]
        warn!("Cloning scene");

        Self {
            path: self.path.clone(),
            objects: self.objects.clone(),
            lights: self.lights.clone(),
            camera: self.camera.clone(),
            settings: self.settings.clone(),
        }
    }
}

struct WithRelativePath<P: AsRef<std::path::Path>>(P);

impl<'de, P: AsRef<std::path::Path> + std::marker::Sync> serde::de::DeserializeSeed<'de>
    for WithRelativePath<P>
{
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
            .par_iter()
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

        // dont fail if extraArgs is missing but warn
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

    #[test]
    fn test_load_scene() -> anyhow::Result<()> {
        // test loading of scene
        let _scene =
            Scene::load("./res/e2e-test/test_config.yaml").context("Failed to load scene")?;

        // test loading of objects
        /*
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
        */

        // test loading of lights
        /*
        assert_eq!(scene.lights.len(), 4);
        assert_eq!(scene.lights[0].position, Point3::new(-2.0, 3.5, -0.8));
        assert_eq!(
            scene.lights[0].color,
            Color::new(0.577_350_3, 0.577_350_3, 0.577_350_3)
        );
        assert!(f32::abs(scene.lights[0].intensity - 40.0) < f32::EPSILON);

        // test loading of camera
        assert_eq!(
            scene.camera.position,
            Point3::new(3.708_024_5, 2.114_768_7, 7.091_919_4)
        );
        assert_eq!(scene.camera.look_at, Point3::new(3.348_594_4, 1.793_123, 6.215_932));
        assert_eq!(scene.camera.up, Vector3::new(0.0, 1.0, 0.0));
        assert!(f32::abs(scene.camera.fov - 63.02536_f32.to_radians()) < f32::EPSILON);
        assert_eq!(scene.camera.resolution, (1920, 1080));
        */

        Ok(())
    }
}
