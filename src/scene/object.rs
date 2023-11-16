use anyhow::Context;
use bvh::bvh::BVH;
use log::warn;
use nalgebra::{Point3, Similarity3, Vector3};
use obj::{Material, SimplePolygon};
use ordered_float::OrderedFloat;

use crate::raytracer::{Hit, Ray};

use super::triangle::Triangle;

#[derive(Debug, Clone)]
pub struct Object {
    pub triangles: Vec<Triangle>,
    pub materials: Vec<Material>,
    pub transform: Similarity3<f32>,
    bvh: BVH,
}

impl<'de> serde::Deserialize<'de> for Object {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        mod yaml {
            use nalgebra::{Point3, Vector3};
            use serde::Deserialize;

            #[derive(Deserialize)]
            pub struct Object {
                pub file_path: String,
                #[serde(with = "super::super::yaml::point3_xyz")]
                pub position: Point3<f32>,
                #[serde(with = "super::super::yaml::vector3_xyz")]
                pub rotation: Vector3<f32>,
                pub scale: f32,
            }
        }

        let yaml_object = yaml::Object::deserialize(deserializer)?;

        let transform = Similarity3::from_parts(
            nalgebra::Translation3::from(yaml_object.position.coords),
            nalgebra::UnitQuaternion::from_euler_angles(
                yaml_object.rotation.x * std::f32::consts::PI * 2.0 / 360.0,
                yaml_object.rotation.y * std::f32::consts::PI * 2.0 / 360.0,
                yaml_object.rotation.z * std::f32::consts::PI * 2.0 / 360.0,
            ),
            yaml_object.scale,
        );

        Object::from_obj(yaml_object.file_path.as_str(), transform)
            .context(format!(
                "Failed to load object from path: {:?}",
                yaml_object.file_path
            ))
            .map_err(serde::de::Error::custom)
    }
}

impl Object {
    fn from_obj<P: AsRef<std::path::Path>>(
        path: P,
        transform: Similarity3<f32>,
    ) -> anyhow::Result<Object> {
        let mut obj = obj::Obj::load(path.as_ref())
            .context(format!("Failed to load obj from path: {:?}", path.as_ref()))?;
        obj.load_mtls().context(format!(
            "Failed to load materials from obj path: {:?}",
            path.as_ref()
        ))?;

        let materials: Vec<Material> = obj
            .data
            .material_libs
            .iter()
            .flat_map(|m| &m.materials)
            .map(|m| m.as_ref().clone())
            .collect::<Vec<_>>();

        let mut triangles = obj
            .data
            .objects
            .iter()
            .flat_map(|object| object.groups.iter())
            .flat_map(|group| {
                let material_index = group
                    .material
                    .as_ref()
                    .map(|m| match m {
                        obj::ObjMaterial::Ref(str) => {
                            panic!("Material reference not supported: {}", str)
                        }
                        obj::ObjMaterial::Mtl(m) => m,
                    })
                    .and_then(|m| {
                        materials
                            .iter()
                            .position(|mat: &Material| mat.name == m.name)
                            .or_else(|| {
                                warn!("Material not found: {}", m.name);
                                None
                            })
                    });

                group
                    .polys
                    .iter()
                    .flat_map(|p| triangulate(&obj, p, material_index))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        let bvh = BVH::build(triangles.as_mut_slice());

        Ok(Object {
            triangles,
            materials,
            transform,
            bvh,
        })
    }

    pub fn intersect(&self, ray: Ray, delta: f32) -> Option<Hit> {
        // Transform ray into object space
        let ray = Ray {
            origin: self.transform.inverse_transform_point(&ray.origin),
            direction: self.transform.inverse_transform_vector(&ray.direction),
        };

        self.bvh
            .traverse(
                &bvh::ray::Ray::new(
                    Into::<[f32; 3]>::into(ray.origin.coords).into(),
                    Into::<[f32; 3]>::into(ray.direction).into(),
                ),
                self.triangles.as_slice(),
            )
            .into_iter()
            .filter_map(|t| t.intersect(ray, delta).map(|h| (t, h)))
            .map(|(t, (u, v, w))| {
                // u, v, w are barycentric coordinates of the hit point on the triangle
                // interpolate hit point and normal
                let point = Point3::from((t.a * u).coords + (t.b * v).coords + (t.c * w).coords);
                let normal = (t.a_normal * u) + (t.b_normal * v) + (t.c_normal * w);

                // Transform hit point and normal back into world space
                Hit {
                    point: self.transform.transform_point(&point),
                    normal: self.transform.transform_vector(&normal),
                    material: t.material_index.map(|i| &self.materials[i]),
                }
            })
            .min_by_key(|h| OrderedFloat((h.point - ray.origin).norm()))
    }
}

fn triangulate(
    obj: &obj::Obj,
    poly: &SimplePolygon,
    material_index: Option<usize>,
) -> Vec<Triangle> {
    let mut triangles = Vec::new();

    for i in 1..poly.0.len() - 1 {
        let a = Point3::from(obj.data.position[poly.0[0].0]);
        let b = Point3::from(obj.data.position[poly.0[i].0]);
        let c = Point3::from(obj.data.position[poly.0[i + 1].0]);

        let computed_normal = (b - a).cross(&(c - a)).normalize();

        triangles.push(Triangle::new(
            a,
            b,
            c,
            poly.0[0]
                .2
                .map(|i| Vector3::from(obj.data.normal[i]))
                .unwrap_or_else(|| {
                    warn!("No normal for vertex: {:?}", poly.0[0]);
                    computed_normal
                }),
            poly.0[i]
                .2
                .map(|i| Vector3::from(obj.data.normal[i]))
                .unwrap_or_else(|| {
                    warn!("No normal for vertex: {:?}", poly.0[i]);
                    computed_normal
                }),
            poly.0[i + 1]
                .2
                .map(|i| Vector3::from(obj.data.normal[i]))
                .unwrap_or_else(|| {
                    warn!("No normal for vertex: {:?}", poly.0[i + 1]);
                    computed_normal
                }),
            material_index,
        ));
    }

    triangles
}
