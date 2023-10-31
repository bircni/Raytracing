use nalgebra::{Point3, Vector3};

#[derive(Debug, Clone, PartialEq)]
pub struct Triangle {
    pub a: Point3<f32>,
    pub b: Point3<f32>,
    pub c: Point3<f32>,
    pub a_normal: Vector3<f32>,
    pub b_normal: Vector3<f32>,
    pub c_normal: Vector3<f32>,
    pub material_index: Option<usize>,
}
