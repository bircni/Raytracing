use nalgebra::{Point3, Vector3};

#[derive(Debug, Clone, PartialEq)]
pub struct Camera {
    position: Point3<f32>,
    direction: Vector3<f32>,
    up: Vector3<f32>,
    fov: f32,
}
