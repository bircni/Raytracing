use super::Color;
use image::RgbImage;

#[derive(Debug, Clone)]
pub struct Material {
    pub name: String,
    pub diffuse_color: Option<Color>,
    pub specular_color: Option<Color>,
    pub specular_exponent: Option<f32>,
    pub diffuse_texture: Option<RgbImage>,
    pub illumination_model: IlluminationModel,
    pub dissolve: Option<f32>,
    #[allow(dead_code)]
    pub refraction_index: Option<f32>,
}

/**
0. Color on and Ambient off
1. Color on and Ambient on
2. Highlight on
3. Reflection on and Ray trace on
4. Transparency: Glass on, Reflection: Ray trace on
5. Reflection: Fresnel on and Ray trace on
6. Transparency: Refraction on, Reflection: Fresnel off and Ray trace on
7. Transparency: Refraction on, Reflection: Fresnel on and Ray trace on
8. Reflection on and Ray trace off
9. Transparency: Glass on, Reflection: Ray trace off
10. Casts shadows onto invisible surfaces
    <http://www.paulbourke.net/dataformats/mtl/> <https://en.wikipedia.org/wiki/Wavefront_.obj_file>
*/
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct IlluminationModel(i32);

impl IlluminationModel {
    pub fn from_i32(i: i32) -> Option<Self> {
        if (0..=10).contains(&i) {
            Some(Self(i))
        } else {
            None
        }
    }

    pub const fn specular(self) -> bool {
        self.0 == 2
    }

    pub const fn reflection(self) -> bool {
        self.0 == 3 || self.0 == 4
    }

    pub const fn transparency(self) -> bool {
        self.0 == 6 || self.0 == 7
    }
}
