use nalgebra::{Point3, Vector3};

use crate::scene::Scene;
//use scene::object::Object;
use std::f64;
use rand::Rng;

use crate::scene::object::Object;


pub struct Ray {
    pub origin: Point3<f64>,
    pub direction: Vector3<f64>,
}
pub struct Intersection {
    pub point: Point3<f64>,
    pub normal: Vector3<f64>,
    pub t: f64,
}

pub struct Raytracer<'a> {
    pub scene: &'a Scene,
    pub width: u32,
    pub height: u32,
    pub fov: f64,
    pub background_color: Vector3<f64>,
    pub max_depth: u32,
}

impl Intersection {
    pub fn new(point: Point3<f64>, normal: Vector3<f64>, t: f64) -> Self {
        Intersection { point, normal, t }
    }
}


impl Raytracer<'_> {
    pub fn trace_ray(&self, ray: &Ray, depth: u32) -> Vector3<f64> {
        if depth > self.max_depth {
            return self.background_color;
        }

        let mut closest_intersection: Option<Intersection> = None;
        let mut closest_object: Option<&Object> = None;

        for object in self.scene.objects.iter() {
            if let Some(intersection) = object.intersect(ray) {
                if closest_intersection.is_none() || intersection.t < closest_intersection.unwrap().t {
                    closest_intersection = Some(intersection);
                    closest_object = Some(object);
                }
            }
        }

        if let Some(intersection) = closest_intersection {
            let object = closest_object.unwrap();
            let mut color = Vector3::new(0.0, 0.0, 0.0);

            for light in self.scene.lights.iter() {
                let light_direction = (light.position - intersection.point).normalize();
                let shadow_ray = Ray {
                    origin: intersection.point + intersection.normal * 0.0001,
                    direction: light_direction,
                };

                let mut in_shadow = false;
                for object in self.scene.objects.iter() {
                    if let Some(shadow_intersection) = object.intersect(&shadow_ray) {
                        if shadow_intersection.t < (light.position - intersection.point).norm() {
                            in_shadow = true;
                            break;
                        }
                    }
                }

                if !in_shadow {
                    let diffuse = object.material.diffuse;
                    let specular = object.material.specular;
                    let shininess = object.material.shininess;

                    let light_intensity = light.intensity / (light.position - intersection.point).norm_squared();
                    let diffuse_intensity = light_intensity * diffuse * intersection.normal.dot(&light_direction).max(0.0);
                    let view_direction = -ray.direction.normalize();
                    let half_vector = (light_direction + view_direction).normalize();
                    let specular_intensity = light_intensity * specular * intersection.normal.dot(&half_vector).max(0.0).powf(shininess);

                    color += light.color.component_mul(&(diffuse_intensity + specular_intensity));
                }
            }

            let reflection_ray = Ray {
                origin: intersection.point + intersection.normal * 0.0001,
                direction: ray.direction - 2.0 * ray.direction.dot(&intersection.normal) * intersection.normal,
            };

            let reflection_color = self.trace_ray(&reflection_ray, depth + 1);
            let reflection_intensity = object.material.reflection;
            color = color * (1.0 - reflection_intensity) + reflection_color * reflection_intensity;

            color
        } else {
            self.background_color
        }
    }

    pub fn render(&self) -> Vec<Vector3<f64>> {
        let mut rng = rand::thread_rng();
        let mut pixels = vec![Vector3::new(0.0, 0.0, 0.0); (self.width * self.height) as usize];

        for y in 0..self.height {
            for x in 0..self.width {
                let u = (x as f64 + rng.gen::<f64>()) / self.width as f64;
                let v = (y as f64 + rng.gen::<f64>()) / self.height as f64;

                let aspect_ratio = self.width as f64 / self.height as f64;
                let fov_adjustment = (self.fov.to_radians() / 2.0).tan();
                let sensor_x = (((x as f64 + 0.5) / self.width as f64) * 2.0 - 1.0) * aspect_ratio * fov_adjustment;
                let sensor_y = (1.0 - ((y as f64 + 0.5) / self.height as f64) * 2.0) * fov_adjustment;

                let direction = Vector3::new(sensor_x, sensor_y, -1.0).normalize();
                let ray = Ray {
                    origin: Point3::new(0.0, 0.0, 0.0),
                    direction: direction,
                };

                let color = self.trace_ray(&ray, 0);
                pixels[(y * self.width + x) as usize] = color;
            }
        }

        pixels
    }
}

