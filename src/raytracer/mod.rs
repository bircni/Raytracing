use image::RgbImage;
use nalgebra::{Point3, Vector2, Vector3};
use ordered_float::OrderedFloat;

use crate::{
    scene::{Material, Scene, Skybox},
    Color,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray {
    pub origin: Point3<f32>,
    pub direction: Vector3<f32>,
}

#[derive(Debug, Clone)]
pub struct Hit<'a> {
    pub name: &'a str,
    pub point: Point3<f32>,
    pub normal: Vector3<f32>,
    pub material: Option<&'a Material>,
    pub uv: Vector2<f32>,
}

pub struct Raytracer {
    scene: Scene,
    delta: f32,
    max_depth: u32,
}

impl Raytracer {
    const NO_MATERIAL_COLOR: Color = Color::new(0.9, 0.9, 0.9);

    pub fn new(scene: Scene, delta: f32, max_depth: u32) -> Raytracer {
        Raytracer {
            scene,
            delta,
            max_depth,
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

    fn refract(incident_ray: Vector3<f32>, surface_normal: Vector3<f32>, eta: f32) -> Vector3<f32> {
        let cos_theta_i = -surface_normal.dot(&incident_ray);
        let sin_theta_t_squared = eta.powi(2) * (1.0 - cos_theta_i.powi(2));
        if sin_theta_t_squared > 1.0 {
            return Vector3::zeros();
        }
        let cos_theta_t = (1.0 - sin_theta_t_squared).sqrt();
        eta * incident_ray + (eta * cos_theta_i - cos_theta_t) * surface_normal
    }

    fn skybox(&self, direction: Vector3<f32>) -> Color {
        let x = (direction.x.atan2(direction.z) / (2.0 * std::f32::consts::PI) + 0.5) % 1.0;
        let y = (direction.y + 0.08).acos() / std::f32::consts::PI;

        match &self.scene.settings.skybox {
            Skybox::Image { image, .. } => {
                let x = (x * image.width() as f32) as u32 % image.width();
                let y = (y * image.height() as f32) as u32 % image.height();

                let pixel = image.get_pixel(x, y);

                Color::new(
                    f32::from(pixel[0]) / 255.0,
                    f32::from(pixel[1]) / 255.0,
                    f32::from(pixel[2]) / 255.0,
                )
            }
            Skybox::Color(color) => *color,
        }
    }

    fn texture(texture: &RgbImage, uv: Vector2<f32>) -> Color {
        let x = (uv.x * texture.width() as f32) as u32 % texture.width();
        let y = ((1.0 - uv.y) * texture.height() as f32) as u32 % texture.height();
        let pixel = texture.get_pixel(x, y);
        Color::new(
            f32::from(pixel[0]) / 255.0,
            f32::from(pixel[1]) / 255.0,
            f32::from(pixel[2]) / 255.0,
        )
    }

    fn raycast_transparent(&self, ray: Ray, max_depth: u32) -> Box<[Hit]> {
        let mut hits = Vec::new();
        let mut ray = ray;
        for _ in 0..max_depth {
            match self.raycast(ray) {
                Some(hit) => {
                    if hit
                        .material
                        .is_some_and(|m| m.dissolve.is_some_and(|d| (d - 1.0).abs() > 0.001))
                    {
                        hits.push(hit);
                        ray.origin += ray.direction * self.delta;
                    } else {
                        hits.push(hit);
                        break;
                    }
                }
                None => break,
            }
        }

        hits.into_boxed_slice()
    }

    fn shade(&self, ray: Ray, depth: u32) -> Color {
        match self
            .raycast_transparent(ray, self.max_depth)
            .to_vec()
            .as_slice()
        {
            [] => self.skybox(ray.direction),
            hits => {
                hits.iter()
                    .fold((Color::zeros(), 1.0), |(color, energy), hit| {
                        (
                            color + self.shade_impl(ray, hit, depth) * energy,
                            energy * (1.0 - hit.material.and_then(|m| m.dissolve).unwrap_or(1.0)),
                        )
                    })
                    .0
            }
        }
    }

    fn shade_impl(&self, ray: Ray, hit: &Hit, depth: u32) -> Color {
        if depth >= self.max_depth {
            return self.skybox(ray.direction);
        }

        let diffuse_color = hit
            .material
            .and_then(|m| m.diffuse_texture.as_ref())
            .map(|map| Self::texture(map, hit.uv))
            .or(hit.material.and_then(|m| m.diffuse_color).map(Color::from))
            .unwrap_or(Self::NO_MATERIAL_COLOR);

        let specular_color = hit
            .material
            .and_then(|m| m.specular_color)
            .map_or(Self::NO_MATERIAL_COLOR, Color::from);

        let ambient_color =
            self.scene.settings.ambient_color * self.scene.settings.ambient_intensity;

        let mut color = ambient_color.component_mul(&diffuse_color);

        for light in &self.scene.lights {
            let light_direction = (light.position - hit.point).normalize();
            let light_ray = Ray {
                origin: hit.point + light_direction * self.delta,
                direction: light_direction,
            };

            let light_transmission_color = self
                .raycast_transparent(light_ray, self.max_depth)
                .iter()
                .fold(Color::new(1.0, 1.0, 1.0), |color, hit| {
                    let diffuse = hit
                        .material
                        .and_then(|m| m.diffuse_color)
                        .unwrap_or(Self::NO_MATERIAL_COLOR)
                        * (1.0 - hit.material.and_then(|m| m.dissolve).unwrap_or(1.0));

                    color.component_mul(&diffuse)
                });

            if light_transmission_color == Color::zeros() {
                continue;
            }

            // diffuse component
            let light_intensity = light.intensity / (light.position - hit.point).norm_squared();
            let light_reflection = light_direction.dot(&hit.normal).max(0.0);
            color += diffuse_color.component_mul(&light_transmission_color)
                * light_intensity
                * light_reflection;

            // specular component
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

                color += specular_color.component_mul(&light_transmission_color)
                    * light_intensity
                    * specular_component;
            }

            // reflection component
            if hit
                .material
                .is_some_and(|m| m.illumination_model.reflection())
            {
                let reflection_direction = Self::reflect(ray.direction, hit.normal);
                let reflection_ray = Ray {
                    origin: hit.point + reflection_direction * self.delta,
                    direction: reflection_direction,
                };

                // compute fresnel based on Schlick's approximation
                let fresnel = 0.04 + 0.96 * (1.0 - reflection_ray.direction.dot(&hit.normal));
                let reflection = self.shade(reflection_ray, depth + 1);

                // mix reflection and diffuse color based on fresnel and specular exponent
                let specular_exponent = hit
                    .material
                    .and_then(|m| m.specular_exponent)
                    .unwrap_or(1.0)
                    / 1000.0;

                color = color.lerp(&reflection, 1.0 - fresnel.powf(specular_exponent));
            }
        }

        color
    }

    /// Render a pixel at the given coordinates.
    /// x and y are in the range 0..width and 0..height
    /// where (0, 0) is the top left corner.
    pub fn render(&self, (x, y): (u32, u32), (width, height): (u32, u32)) -> Color {
        let x = ((x as f32 / width as f32) * 2.0 - 1.0) * (width as f32 / height as f32);
        let y = (y as f32 / height as f32) * 2.0 - 1.0;

        let ray = self.scene.camera.ray(x, y);
        self.shade(ray, 0)
    }
}
