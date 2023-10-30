use anyhow::Context;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ModelConfig {
    file_path: String,
    position: HashMap<String, f32>,
    rotation: HashMap<String, f32>,
    scale: f32,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PointLightConfig {
    position: HashMap<String, f32>,
    ke: HashMap<String, f32>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct CameraConfig {
    position: HashMap<String, f32>,
    look_at: HashMap<String, f32>,
    up_vec: HashMap<String, f32>,
    field_of_view: f32,
    width: u32,
    height: u32,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ExtraArgsConfig {
    max_bounces: u32,
    samples: u32,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct AppConfig {
    pub title: String,
    pub model: ModelConfig,
    pub point_lights: Vec<PointLightConfig>,
    pub camera: CameraConfig,
    pub extra_args: ExtraArgsConfig,
}

impl AppConfig {
    pub fn load<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<AppConfig> {
        let s = std::fs::read_to_string(path.as_ref()).context(format!(
            "Failed to read file from path: {}",
            path.as_ref().display()
        ))?;

        serde_yaml::from_str(s.as_str()).context("Failed to parse config file")
    }
}

#[test]
pub fn test_load() -> anyhow::Result<()> {
    let config = AppConfig::load("./res/config.yaml")?;
    assert!(config.title == "Raytracer");
    Ok(())
}
