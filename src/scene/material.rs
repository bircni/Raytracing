use image::RgbImage;

use crate::Color;

#[derive(Debug, Clone)]
pub struct Material {
    pub name: String,
    pub diffuse_color: Option<Color>,
    pub specular_color: Option<Color>,
    pub specular_exponent: Option<f32>,
    pub diffuse_texture: Option<RgbImage>,
    pub illumination_model: IlluminationModel,
    pub dissolve: Option<f32>,
    pub refraction_index: Option<f32>,
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct IlluminationModel(i32);

impl IlluminationModel {
    pub fn from_i32(i: i32) -> Option<IlluminationModel> {
        if (0..=10).contains(&i) {
            Some(IlluminationModel(i))
        } else {
            None
        }
    }

    pub fn specular(self) -> bool {
        self.0 == 2
    }

    pub fn reflection(self) -> bool {
        self.0 == 3 || self.0 == 4
    }

    pub fn transparency(self) -> bool {
        self.0 == 6 || self.0 == 7
    }
}
