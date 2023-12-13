use bvh::{aabb::Bounded, bounding_hierarchy::BHShape};
use nalgebra::{Point3, Vector2, Vector3};

use crate::raytracer::Ray;

#[derive(Debug, Clone, PartialEq)]
pub struct Triangle {
    pub a: Point3<f32>,
    pub b: Point3<f32>,
    pub c: Point3<f32>,
    pub a_normal: Vector3<f32>,
    pub b_normal: Vector3<f32>,
    pub c_normal: Vector3<f32>,
    pub a_uv: Vector2<f32>,
    pub b_uv: Vector2<f32>,
    pub c_uv: Vector2<f32>,
    pub material_index: Option<usize>,
    bvh_index: usize,
}

impl Triangle {
    //quick fix for linter
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        a: Point3<f32>,
        b: Point3<f32>,
        c: Point3<f32>,
        a_normal: Vector3<f32>,
        b_normal: Vector3<f32>,
        c_normal: Vector3<f32>,
        a_uv: Vector2<f32>,
        b_uv: Vector2<f32>,
        c_uv: Vector2<f32>,
        material_index: Option<usize>,
    ) -> Self {
        Self {
            a,
            b,
            c,
            a_normal,
            b_normal,
            c_normal,
            a_uv,
            b_uv,
            c_uv,
            material_index,
            bvh_index: 0,
        }
    }

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

impl Bounded<f32, 3> for Triangle {
    fn aabb(&self) -> bvh::aabb::Aabb<f32, 3> {
        bvh::aabb::Aabb::empty()
            .grow(&self.a)
            .grow(&self.b)
            .grow(&self.c)
    }
}

impl BHShape<f32, 3> for Triangle {
    fn set_bh_node_index(&mut self, index: usize) {
        self.bvh_index = index;
    }

    fn bh_node_index(&self) -> usize {
        self.bvh_index
    }
}
