use serde::{Serialize, Deserialize};

use crate::Color;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Skybox {
    Color(Color),
}

impl Skybox {
    pub fn new_color(color: Color) -> Self {
        Self::Color(color)
    }
}