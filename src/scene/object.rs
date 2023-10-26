use nalgebra::{Translation3, UnitQuaternion};

use super::triangle::Triangle;

#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    triangles: Vec<Triangle>,
    materials: Vec<obj::Material>,
    translation: Translation3<f32>,
    rotation: UnitQuaternion<f32>,
}
