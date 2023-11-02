use nalgebra::{Point3, Vector3};

#[derive(Debug, Clone, PartialEq)]
pub struct Triangle {
    a: Point3<f32>,
    b: Point3<f32>,
    c: Point3<f32>,
    a_normal: Vector3<f32>,
    b_normal: Vector3<f32>,
    c_normal: Vector3<f32>,
    material_index: usize,
}
