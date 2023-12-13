use nalgebra::{Point3, Vector2, Vector3};
use ordered_float::OrderedFloat;

use crate::{
    scene::{Material, Scene},
    Color,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray {
    pub origin: Point3<f32>,
    pub direction: Vector3<f32>,
}

#[derive(Debug)]
pub struct Hit<'a> {
    pub name: &'a str,
    pub direction: Vector3<f32>,
    pub point: Point3<f32>,
    pub normal: Vector3<f32>,
    pub material: Option<&'a Material>,
    pub uv: Vector2<f32>,
}

pub struct Raytracer {
    scene: Scene,
    background_color: Color,
    delta: f32,
    max_depth: u32,
}

impl Raytracer {
    const NO_MATERIAL_COLOR: Color = Color::new(0.9, 0.9, 0.9);

    pub fn new(scene: Scene, delta: f32) -> Raytracer {
        Raytracer {
            background_color: scene.settings.background_color,
            scene,
            delta,
            max_depth: 5,
        }
    }

    fn raycast(&self, ray: Ray) -> Option<Hit> {
        self.scene
            .objects
            .iter()
            .filter_map(|o| o.intersect(ray, self.delta))
            .min_by_key(|h| OrderedFloat((h.point - ray.origin).norm()))
    }

    fn reflect(incoming: Vector3<f32>, normal: Vector3<f32>) -> Vector3<f32> {
        incoming - 2.0 * incoming.dot(&normal) * normal
    }

    fn shade(&self, hit: Option<Hit>, depth: u32) -> Color {
        if let Some(hit) = hit {
            let diffuse = (hit
                .material
                .and_then(|m| m.diffuse_texture.as_ref())
                .map(|map| {
                    let uv = hit.uv;
                    let x = (uv.x * map.width() as f32) as u32 % map.width();
                    let y = ((1.0 - uv.y) * map.height() as f32) as u32 % map.height();
                    let pixel = map.get_pixel(x, y);
                    Color::new(
                        f32::from(pixel[0]) / 255.0,
                        f32::from(pixel[1]) / 255.0,
                        f32::from(pixel[2]) / 255.0,
                    )
                }))
            .or(hit.material.and_then(|m| m.diffuse_color).map(Color::from))
            .unwrap_or(Self::NO_MATERIAL_COLOR);
            let specular = hit
                .material
                .and_then(|m| m.specular_color)
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
                    // Diffuse
                    let light_intensity =
                        light.intensity / (light.position - hit.point).norm_squared();
                    let light_reflection = light_direction.dot(&hit.normal).max(0.0);

                    color +=
                        diffuse.component_mul(&light.color) * light_intensity * light_reflection;

                    // Specular
                    if hit
                        .material
                        .is_some_and(|m| m.illumination_model.specular())
                    {
                        let reflection_direction = Self::reflect(-light_direction, hit.normal);
                        let specular_component = reflection_direction
                            .dot(&-light_ray.direction)
                            .max(0.0)
                            .powf(
                                hit.material
                                    .and_then(|m| m.specular_exponent)
                                    .unwrap_or(1.0),
                            );

                        color += specular.component_mul(&light.color)
                            * light_intensity
                            * specular_component;
                    }
                }

                // Reflection
                if depth < self.max_depth
                    && hit
                        .material
                        .is_some_and(|m| m.illumination_model.reflection())
                {
                    let reflection_direction = Self::reflect(hit.direction, hit.normal);
                    let reflection_ray = Ray {
                        origin: hit.point + reflection_direction * self.delta,
                        direction: reflection_direction,
                    };

                    color = self
                        .shade(self.raycast(reflection_ray), depth + 1)
                        .component_mul(&color)
                        * hit
                            .material
                            .and_then(|m| m.specular_exponent)
                            .unwrap_or(1000.0)
                        / 1000.0;
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
        self.shade(hit, 0)
    }
}
