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
    pub fn intersect(&self, ray: Ray, delta: f32) -> Option<(f32, f32, f32)> {
        let ab = self.b - self.a;
        let ac = self.c - self.a;
        let normal = ab.cross(&ac).normalize();

        let t = (self.a - ray.origin).dot(&normal) / ray.direction.dot(&normal);

        if t < delta {
            return None;
        }

        let p = ray.origin + ray.direction * t;

        let ap = p - self.a;
        let bp = p - self.b;
        let cp = p - self.c;

        let ab_ap = ab.cross(&ap);
        let bc_bp = (self.c - self.b).cross(&bp);
        let ca_cp = (self.a - self.c).cross(&cp);

        let ab_ap_dot = ab_ap.dot(&normal);
        let bc_bp_dot = bc_bp.dot(&normal);
        let ca_cp_dot = ca_cp.dot(&normal);

        if ab_ap_dot < 0.0 || bc_bp_dot < 0.0 || ca_cp_dot < 0.0 {
            return None;
        }

        let area = ab_ap_dot + bc_bp_dot + ca_cp_dot;

        Some((bc_bp_dot / area, ca_cp_dot / area, ab_ap_dot / area))
    }
}
