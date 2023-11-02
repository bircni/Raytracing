use anyhow::Context;
use log::warn;
use nalgebra::{Point3, Similarity3, Vector3};
use obj::{Material, SimplePolygon};
use ordered_float::OrderedFloat;

use crate::raytracer::{Hit, Ray};

use super::triangle::Triangle;

#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    pub triangles: Vec<Triangle>,
    pub materials: Vec<Material>,
    pub transform: Similarity3<f32>,
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

        let triangles = obj
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

        Ok(Object {
            triangles,
            materials,
            transform,
        })
    }
}

fn triangulate(
    obj: &obj::Obj,
    poly: &SimplePolygon,
    material_index: Option<usize>,
) -> Vec<Triangle> {
    let mut triangles = Vec::new();

    for i in 1..poly.0.len() - 1 {
        triangles.push(Triangle {
            a: Point3::from(obj.data.position[poly.0[0].0]),
            b: Point3::from(obj.data.position[poly.0[i].0]),
            c: Point3::from(obj.data.position[poly.0[i + 1].0]),
            a_normal: poly.0[0]
                .2
                .map(|i| Vector3::from(obj.data.normal[i]))
                .unwrap_or(Vector3::zeros()),
            b_normal: poly.0[i]
                .2
                .map(|i| Vector3::from(obj.data.normal[i]))
                .unwrap_or(Vector3::zeros()),
            c_normal: poly.0[i + 1]
                .2
                .map(|i| Vector3::from(obj.data.normal[i]))
                .unwrap_or(Vector3::zeros()),
            material_index,
        });
    }

    triangles
}

impl Object {
    pub fn intersect(&self, ray: Ray) -> Option<Hit> {
        // Transform ray into object space
        let ray = Ray {
            origin: self.transform.inverse_transform_point(&ray.origin),
            direction: self.transform.inverse_transform_vector(&ray.direction),
        };

        self.triangles
            .iter()
            .filter_map(|t| t.intersect(ray).map(|h| (t, h)))
            .map(|(t, (u, v, w))| {
                let material = t.material_index.map(|i| &self.materials[i]);
                let normal = ((t.a_normal * u) + (t.b_normal * v) + (t.c_normal * w)).normalize();
                let point = Point3::from((t.a.coords * u) + (t.b.coords * v) + (t.c.coords * w));
                Hit {
                    point,
                    normal,
                    material,
                }
            })
            .min_by_key(|h| OrderedFloat((h.point - ray.origin).norm()))
            .map(|h| {
                // Transform hit back into world space
                Hit {
                    point: self.transform.transform_point(&h.point),
                    normal: self.transform.transform_vector(&h.normal),
                    ..h
                }
            })
    }
}
