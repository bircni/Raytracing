use self::{camera::Camera, light::Light, object::Object, settings::Settings};

mod camera;
mod light;
mod object;
mod settings;
mod triangle;

#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    objects: Vec<Object>,
    lights: Vec<Light>,
    camera: Camera,
    settings: Settings,
}
