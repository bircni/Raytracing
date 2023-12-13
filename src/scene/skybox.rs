use crate::Color;

#[derive(Debug, Clone, PartialEq)]
pub enum Skybox {
    Color(Color),
}

impl Skybox {
    pub fn new_color(color: Color) -> Self {
        Self::Color(color)
    }
    
    pub fn get_background_color(&self) -> Color {
        match self {
            Skybox::Color(color) => *color,
        }
    }
}