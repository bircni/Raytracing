use image::RgbImage;

use crate::Color;


#[derive(Debug, Clone, PartialEq)]
pub struct Skybox {
    color: Color,
}

impl Skybox {
    pub fn new(color: Color) -> Skybox {
        Skybox { color }
    }

    pub fn get_background_color(&self) -> Color {
        self.color
    }

    /*
    pub fn get_background_texture(&self) -> Option<RgbImage> {
        self.texture.clone()
    }*/
}