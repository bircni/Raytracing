use std::{
    fs::{create_dir_all, File},
    io::{Read, Write},
};

use image::RgbImage;
use nalgebra::{Point3, Vector2, Vector3};
use ordered_float::OrderedFloat;
use strum_macros::EnumIter;

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
    pub point: Point3<f32>,
    pub normal: Vector3<f32>,
    pub material: Option<&'a Material>,
    pub uv: Vector2<f32>,
}

pub struct Raytracer {
    scene: Scene,
    delta: f32,
    max_depth: u32,
    skybox_image: Option<RgbImage>,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy, EnumIter)]
pub enum Skybox {
    None,
    ScythianTombs2,
    RainforestTrail,
    StudioSmall08,
    Kloppenheim02,
    CircusArena,
}

impl Skybox {
    pub fn get_url(&self) -> Option<&str> {
        match self {
            Skybox::None => None,
            Skybox::ScythianTombs2 => {
                Some("https://dl.polyhaven.org/file/ph-assets/HDRIs/exr/4k/scythian_tombs_puresky_4k.exr")
            }
            Skybox::RainforestTrail => {
                Some("https://dl.polyhaven.org/file/ph-assets/HDRIs/exr/4k/rainforest_trail_4k.exr")
            }
            Skybox::StudioSmall08 => {
                Some("https://dl.polyhaven.org/file/ph-assets/HDRIs/exr/4k/studio_small_08_4k.exr")
            }
            Skybox::Kloppenheim02 => {
                Some("https://dl.polyhaven.org/file/ph-assets/HDRIs/exr/4k/kloppenheim_02_4k.exr")
            }
            Skybox::CircusArena => {
                Some("https://dl.polyhaven.org/file/ph-assets/HDRIs/exr/4k/circus_arena_4k.exr")
            }
        }
    }
}

impl std::fmt::Display for Skybox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Skybox::None => write!(f, "None"),
            Skybox::ScythianTombs2 => write!(f, "Scythian Tombs"),
            Skybox::RainforestTrail => write!(f, "Rainforest Trail"),
            Skybox::StudioSmall08 => write!(f, "Studio Small"),
            Skybox::Kloppenheim02 => write!(f, "Kloppenheim"),
            Skybox::CircusArena => write!(f, "Circus Arena"),
        }
    }
}

impl Raytracer {
    const NO_MATERIAL_COLOR: Color = Color::new(0.9, 0.9, 0.9);

    pub fn new(scene: Scene, delta: f32) -> Raytracer {
        Raytracer {
            scene,
            delta,
            max_depth: 5,
            skybox_image: None,
        }
    }

    pub fn load_skybox<P: AsRef<std::path::Path>>(
        &mut self,
        skybox: Skybox,
        save_path: P,
    ) -> anyhow::Result<()> {
        if skybox == Skybox::None {
            return Ok(());
        }
        if let Err(e) = create_dir_all(&save_path) {
            log::error!("Failed to create directory: {}", e);
            return Err(anyhow::anyhow!("Failed to create directory: {}", e));
        }
        let file_path = format!("{}/{}.exr", save_path.as_ref().display(), skybox.clone());
        if let Ok(mut file) = File::open(&file_path) {
            log::info!("Loading skybox from file: {}", file_path);
            let mut img_bytes = Vec::new();
            if file.read_to_end(&mut img_bytes).is_ok() {
                log::info!("Loaded skybox from file: {}", file_path);
                if let Ok(image) = image::load_from_memory(&img_bytes) {
                    self.skybox_image = Some(image.to_rgb8());
                    return Ok(());
                }
            }
            log::error!("Failed to read skybox file: {}", file_path);
            return Err(anyhow::anyhow!("Failed to read skybox file: {}", file_path));
        } else if let Some(url) = skybox.get_url() {
            log::info!("Downloading skybox from: {}", url);
            match reqwest::blocking::get(url) {
                Ok(response) => match response.bytes() {
                    Ok(img_bytes) => {
                        if let Ok(mut file) = File::create(&file_path) {
                            if file.write_all(&img_bytes).is_ok() {
                                log::info!("Downloaded and saved skybox to: {}", file_path);
                                if let Ok(image) = image::load_from_memory(&img_bytes) {
                                    self.skybox_image = Some(image.to_rgb8());
                                    return Ok(());
                                }
                            }
                        }
                        return Err(anyhow::anyhow!(
                            "Failed to save skybox to file: {}",
                            file_path
                        ));
                    }
                    Err(e) => {
                        log::error!("Failed to get skybox bytes: {}", e);
                        return Err(anyhow::anyhow!("Failed to get skybox bytes: {}", e));
                    }
                },
                Err(e) => {
                    log::error!("Failed to download skybox: {}", e);
                    return Err(anyhow::anyhow!("Failed to download skybox: {}", e));
                }
            }
        }
        Err(anyhow::anyhow!("Failed to get skybox url"))
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

    fn shade(&self, ray: Ray, hit: Option<Hit>, depth: u32) -> Color {
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
                    let reflection_direction = Self::reflect(ray.direction, hit.normal);
                    let reflection_ray = Ray {
                        origin: hit.point + reflection_direction * self.delta,
                        direction: reflection_direction,
                    };

                    // compute fresnel based on Schlick's approximation
                    let fresnel = 0.04 + 0.96 * (1.0 - reflection_ray.direction.dot(&hit.normal));
                    let reflection =
                        self.shade(reflection_ray, self.raycast(reflection_ray), depth + 1);

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
        } else {
            // environment map projection
            let ray_dir = ray.direction.normalize();
            let x = 0.5 + ray_dir.z.atan2(ray_dir.x) / (2.0 * std::f32::consts::PI);
            let y = 0.5 - ray_dir.y.asin() / std::f32::consts::PI;

            if let Some(skybox) = &self.skybox_image {
                let pixel = skybox.get_pixel(
                    (x * skybox.width() as f32) as u32 % skybox.width(),
                    (y * skybox.height() as f32) as u32 % skybox.height(),
                );

                Color::new(
                    f32::from(pixel[0]) / 255.0,
                    f32::from(pixel[1]) / 255.0,
                    f32::from(pixel[2]) / 255.0,
                )
            } else {
                self.scene.settings.background_color
            }
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
        self.shade(ray, hit, 0)
    }
}
