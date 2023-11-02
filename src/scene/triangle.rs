use nalgebra::{Point3, Vector3};

use crate::raytracer::Ray;

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

impl Triangle {
    /// return barycentric coordinates if ray intersects triangle
    pub fn intersect(&self, ray: Ray) -> Option<(f32, f32, f32)> {
        let edge1 = self.b - self.a;
        let edge2 = self.c - self.a;
        let h = ray.direction.cross(&edge2);
        let a = edge1.dot(&h);

        if a.abs() < 1e-8 {
            return None; // This ray is parallel to this triangle.
        }

        let f = 1.0 / a;
        let s = ray.origin - self.a;
        let u = f * s.dot(&h);

        if u < 0.0 || u > 1.0 {
            return None; // The intersection point is outside the triangle.
        }

        let q = s.cross(&edge1);
        let v = f * ray.direction.dot(&q);

        if v < 0.0 || u + v > 1.0 {
            return None; // The intersection point is outside the triangle.
        }

        let t = f * edge2.dot(&q);

        if t > 1e-8 {
            let w = 1.0 - u - v;

            Some((u, v, w))
        } else {
            None
        }
    }
}
