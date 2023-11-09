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

pub struct Raytracer {
    scene: Scene,
    background_color: Color,
}

impl Raytracer {
    pub fn new(scene: Scene, background_color: Vector3<f32>) -> Raytracer {
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

    fn shade(&self, hit: Option<Hit>) -> Color {
        hit.map(|h| {
            h.material
                .and_then(|m| m.kd)
                .map(Color::from)
                .unwrap_or(Color::new(0.9, 0.9, 0.9))
        })
        .unwrap_or(self.background_color)
    }

    pub fn render(&self, (x, y): (usize, usize), (width, height): (usize, usize)) -> Color {
        let ray = self.scene.camera.ray((x, y), (width, height));
        let hit = self.raycast(ray);
        self.shade(hit)
    }
}
