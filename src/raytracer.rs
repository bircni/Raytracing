use nalgebra::{Point3, Vector3};
use obj::Material;
use ordered_float::OrderedFloat;

use crate::{scene::Scene, Color};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray {
    pub origin: Point3<f32>,
    pub direction: Vector3<f32>,
}

#[derive(Debug, PartialEq)]
pub struct Hit<'a> {
    pub point: Point3<f32>,
    pub normal: Vector3<f32>,
    pub material: Option<&'a Material>,
}

pub struct Raytracer<'a> {
    scene: &'a Scene,
    background_color: Color,
}

impl<'a> Raytracer<'a> {
    pub fn new(scene: &'a Scene, background_color: Vector3<f32>) -> Raytracer<'a> {
        Raytracer {
            scene,
            background_color,
        }
    }

    fn raycast(&self, ray: Ray) -> Option<Hit> {
        self.scene
            .objects
            .iter()
            .filter_map(|o| o.intersect(ray))
            .min_by_key(|h| OrderedFloat((h.point - ray.origin).norm()))
    }

    pub fn render(&self, ray: Ray) -> Color {
        self.raycast(ray)
            .map(|_| Color::new(1.0, 0.0, 0.0))
            .unwrap_or(self.background_color)
    }
}
