use anyhow::Context;
use serde::{Deserialize, Serialize};

pub use self::{
    camera::Camera, light::Light, material::Material, object::Object, settings::Settings,
};

mod camera;
mod light;
mod material;
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
