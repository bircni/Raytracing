use anyhow::Context;
use log::warn;
use nalgebra::{Point, Similarity3, Vector3};
use obj::Material;

use super::triangle::Triangle;

#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    triangles: Vec<Triangle>,
    materials: Vec<Material>,
    transform: Similarity3<f32>,
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
                yaml_object.rotation.x,
                yaml_object.rotation.y,
                yaml_object.rotation.z,
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
                    .map(|poly| {
                        let pos1 = obj.data.position[poly.0[0].0];
                        let pos2 = obj.data.position[poly.0[1].0];
                        let pos3 = obj.data.position[poly.0[2].0];
                        let normal1 = obj.data.normal[poly.0[0].2.unwrap()];
                        let normal2 = obj.data.normal[poly.0[1].2.unwrap()];
                        let normal3 = obj.data.normal[poly.0[2].2.unwrap()];

                        Triangle {
                            a: Point::from(pos1),
                            b: Point::from(pos2),
                            c: Point::from(pos3),
                            a_normal: Vector3::from(normal1),
                            b_normal: Vector3::from(normal2),
                            c_normal: Vector3::from(normal3),
                            material_index,
                        }
                    })
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
