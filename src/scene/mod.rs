use self::{camera::Camera, light::Light, object::Object, settings::AppConfig};

mod camera;
mod light;
mod object;
pub mod settings;
mod triangle;

#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    objects: Vec<Object>,
    lights: Vec<Light>,
    camera: Camera,
    settings: AppConfig,
}
