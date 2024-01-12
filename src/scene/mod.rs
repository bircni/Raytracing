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

// read scene config from yaml
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
            .get("extra_args")
            .map(Settings::deserialize)
            .transpose()
            .map_err(|e| {
                warn!("Failed to deserialize extra_args: {}", e);
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
