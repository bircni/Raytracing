use serde::Deserialize;

#[derive(Debug, Clone, PartialEq)]
struct ModelConfig {
    filePath: String,
    position: Point3<f32>,
    rotation: Vector3<f32>,
    scale: f32,
}

#[derive(Debug, Clone, PartialEq)]
struct PointLightConfig {
    position: Point3<f32>,
    Ke: Color,
}

#[derive(Debug, Clone, PartialEq)]
struct CameraConfig {
    position: Point3<f32>,
    lookAt: Point3<f32>,
    upVec: Vector3<f32>,
    fieldOfView: f32,
    width: u32,
    height: u32,
}
    
#[derive(Debug, Clone, PartialEq)]
struct ExtraArgsConfig {
    max_bounces: u32,
    samples: u32,
}

#[derive(Debug, Clone, PartialEq)]
struct AppConfig {
    title: String,
    model: ModelConfig,
    pointLights: Vec<PointLightConfig>,
    camera: CameraConfig,
    extraArgs: ExtraArgsConfig,
}