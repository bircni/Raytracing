// Import necessary Rust crates and modules
use nalgebra::{Point3, Vector3}; // 3D point and vector data types from nalgebra

// Define a custom struct 'Triangle' for representing a 3D triangle
#[derive(Debug, Clone, PartialEq)]
pub struct Triangle {
    pub a: Point3<f32>,      // Vertex 'a' of the triangle
    pub b: Point3<f32>,      // Vertex 'b' of the triangle
    pub c: Point3<f32>,      // Vertex 'c' of the triangle
    pub a_normal: Vector3<f32>, // Normal vector at vertex 'a'
    pub b_normal: Vector3<f32>, // Normal vector at vertex 'b'
    pub c_normal: Vector3<f32>, // Normal vector at vertex 'c'
    pub material_index: Option<usize>, // Optional material index associated with the triangle
}
