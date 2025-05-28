use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Context;
use log::warn;
use nalgebra::Vector3;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{
    Deserialize, Serialize,
    de::{DeserializeSeed, Error, Unexpected},
};

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

struct WithRelativePath<P: AsRef<Path>>(P);

impl<'de, P: AsRef<Path> + Sync> DeserializeSeed<'de> for WithRelativePath<P> {
    type Value = Scene;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let map = <serde_yml::Value as serde::Deserialize>::deserialize(deserializer)?;

        let objects = map
            .get("models")
            .ok_or_else(|| Error::missing_field("models"))?
            .as_sequence()
            .ok_or_else(|| Error::invalid_type(Unexpected::Map, &"a sequence"))?
            .par_iter()
            .map(|v| object::WithRelativePath(self.0.as_ref()).deserialize(v))
            .collect::<Result<Vec<Object>, serde_yml::Error>>()
            .map_err(Error::custom)?;

        let lights = map
            .get("pointLights")
            .ok_or_else(|| Error::missing_field("pointLights"))?
            .as_sequence()
            .ok_or_else(|| Error::invalid_type(Unexpected::Map, &"a sequence"))?
            .iter()
            .map(Light::deserialize)
            .collect::<Result<Vec<Light>, serde_yml::Error>>()
            .map_err(Error::custom)?;

        let camera = map
            .get("camera")
            .ok_or_else(|| Error::missing_field("camera"))?;
        let camera = Camera::deserialize(camera).map_err(Error::custom)?;

        // dont fail if extraArgs is missing but warn
        let settings = map
            .get("extraArgs")
            .map(Settings::deserialize)
            .transpose()
            .map_err(|e| {
                warn!("Failed to deserialize extraArgs: {e}");
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
    pub fn load<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let s = fs::read_to_string(path.as_ref()).context(format!(
            "Failed to read file from path: {}",
            path.as_ref().display()
        ))?;

        WithRelativePath(path.as_ref())
            .deserialize(serde_yml::Deserializer::from_str(&s))
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to deserialize scene from path: {}\n{}",
                    path.as_ref().display(),
                    e
                )
            })
    }
}
