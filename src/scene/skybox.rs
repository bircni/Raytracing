use crate::Color;

pub enum Skybox {
    Color(Color),
}

impl Skybox {
    pub fn new_color(color: Color) -> Self {
        Self::Color(color)
    }
}