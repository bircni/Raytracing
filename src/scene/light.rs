use nalgebra::Point3;

use crate::Color;

#[derive(Debug, Clone, PartialEq)]
pub struct Light {
    position: Point3<f32>,
    color: Color,
    intensity: f32,
}
