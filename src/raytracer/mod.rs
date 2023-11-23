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
    delta: f32,
}

impl Raytracer {
    const NO_MATERIAL_COLOR: Color = Color::new(0.9, 0.9, 0.9);

    pub fn new(scene: Scene, delta: f32) -> Raytracer {
        Raytracer {
            background_color: scene.settings.background_color,
            scene,
            delta,
        }
    }

    fn raycast(&self, ray: Ray) -> Option<Hit> {
        self.scene
            .objects
            .iter()
            .filter_map(|o| o.intersect(ray, self.delta))
            .min_by_key(|h| OrderedFloat((h.point - ray.origin).norm()))
    }

    fn shade(&self, hit: Option<Hit>) -> Color {
        if let Some(hit) = hit {
            let diffuse = hit
                .material
                .and_then(|m| m.kd)
                .map_or(Self::NO_MATERIAL_COLOR, Color::from);

            let mut color = self.scene.settings.ambient_color.component_mul(&diffuse)
                * self.scene.settings.ambient_intensity;

            for light in &self.scene.lights {
                let light_direction = (light.position - hit.point).normalize();
                let light_ray = Ray {
                    origin: hit.point + light_direction * self.delta,
                    direction: light_direction,
                };

                let shadow = self.raycast(light_ray).is_some();

                if !shadow {
                    let light_intensity =
                        light.intensity / (light.position - hit.point).norm_squared();
                    let light_reflection = light_direction.dot(&hit.normal).max(0.0);
                    color +=
                        diffuse.component_mul(&light.color) * light_intensity * light_reflection;
                }
            }

            color
        } else {
            self.background_color
        }
    }

    /// Render a pixel at the given coordinates.
    /// x and y are in the range 0..width and 0..height
    /// where (0, 0) is the top left corner.
    pub fn render(&self, (x, y): (u32, u32), (width, height): (u32, u32)) -> Color {
        let x = ((x as f32 / width as f32) * 2.0 - 1.0) * (width as f32 / height as f32);
        let y = (y as f32 / height as f32) * 2.0 - 1.0;

        let ray = self.scene.camera.ray(x, y);
        let hit = self.raycast(ray);
        self.shade(hit)
    }
}
