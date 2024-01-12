use image::RgbImage;
use nalgebra::{Point3, Vector2, Vector3};
use ordered_float::OrderedFloat;

use crate::{
    scene::{Material, Scene, Skybox},
    Color,
};

/// defines a ray with an origin and a direction
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

// implement raytracer, define methods and variables
impl Raytracer {
    const NO_MATERIAL_COLOR: Color = Color::new(0.9, 0.9, 0.9);

    pub fn new(scene: Scene, delta: f32, max_depth: u32) -> Raytracer {
        Raytracer {
            scene,
            delta,
            max_depth,
        }
    }

    // find the closest intersection of the ray with the scene
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
        let direction = direction
            .try_normalize(f32::EPSILON)
            .unwrap_or(Vector3::y());

        let x = 0.5 + direction.z.atan2(direction.x) / (2.0 * std::f32::consts::PI);
        let y = 0.5 - direction.y.asin() / std::f32::consts::PI;

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

    fn raycast_transparent(&self, ray: Ray) -> Box<[Hit]> {
        let mut hits = Vec::<Hit>::new();
        let mut ray = ray;

        while let Some(hit) = self.raycast(ray) {
            hits.push(hit.clone());

            if let Some(material) = hit.material {
                if material.illumination_model.transparency() {
                    // hochwissenschaftliche Formel
                    ray.origin += ray.direction * 0.05;
                    continue;
                }
            }
            break;
        }

        hits.into_boxed_slice()
    }

    // calculate shading for a given hit point based on ambient and diffuse lighting
    fn shade(&self, ray: Ray, depth: u32) -> Color {
        // hochwissnschaftliche Formel +- x
        self.raycast_transparent(ray).last().map_or_else(
            || self.skybox(ray.direction),
            |hit| self.shade_impl(ray, hit, depth),
        )
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

        let mut color = self
            .scene
            .settings
            .ambient_color
            .component_mul(&diffuse_color)
            * self.scene.settings.ambient_intensity;

        for light in &self.scene.lights {
            let light_direction = (light.position - hit.point).normalize();
            let light_ray = Ray {
                origin: hit.point + light_direction * self.delta,
                direction: light_direction,
            };

            let light_transmission_color = self
                .raycast_transparent(light_ray)
                .iter()
                .last()
                .map_or(Color::from_element(1.0), |hit| {
                    color.component_mul(
                        &hit.material
                            .and_then(|m| m.diffuse_color)
                            .unwrap_or(Color::from_element(1.0)),
                    ) * hit.material.and_then(|m| m.dissolve).unwrap_or(1.0)
                })
                .component_mul(&light.color);

            if light_transmission_color.norm() < 0.01 {
                continue;
            }

            // diffuse component
            let light_intensity = light.intensity / (light.position - hit.point).norm_squared();
            let diffuse_intensity = light_direction.dot(&hit.normal).max(0.0) * light_intensity;
            color += diffuse_color.component_mul(&light_transmission_color) * diffuse_intensity;

            // specular component
            if hit
                .material
                .is_some_and(|m| m.illumination_model.specular())
            {
                let specular_intensity = light_direction
                    .dot(&Self::reflect(-ray.direction, hit.normal))
                    .max(0.0)
                    .powf(
                        hit.material
                            .and_then(|m| m.specular_exponent)
                            .unwrap_or(1.0),
                    )
                    * light_intensity;
                color +=
                    specular_color.component_mul(&light_transmission_color) * specular_intensity;
            }

            // reflection
            if hit
                .material
                .is_some_and(|m| m.illumination_model.reflection())
            {
                let reflection_ray = Ray {
                    origin: hit.point + hit.normal * self.delta,
                    direction: Self::reflect(ray.direction, hit.normal),
                };
                color = color.component_mul(&self.shade(reflection_ray, depth + 1));
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
