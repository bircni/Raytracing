use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;

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

pub fn load_configuration() -> Result<AppConfig, Box<dyn std::error::Error>> {
    let file_path = "./config.yaml";
    let mut file = File::open(file_path)?;

    let mut config_data = String::new();
    file.read_to_string(&mut config_data)?;

    // Deserialize the YAML data into an AppConfig struct
    let app_config: AppConfig = serde_yaml::from_str(&config_data)?;

    // Check for errors during deserialization
    if let Err(err) = serde_yaml::from_str::<AppConfig>(&config_data) {
        eprintln!("Error deserializing YAML: {:?}", err);
        return Err(err.into());
    }

    print!("{:?}", app_config);
    // Return the deserialized configuration
    Ok(app_config)
}