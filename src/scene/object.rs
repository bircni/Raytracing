use super::{
    Color,
    material::{IlluminationModel, Material},
    triangle::Triangle,
};
use crate::raytracer::{Hit, Ray};
use anyhow::Context;
use bvh::{bvh::Bvh, ray};
use image::RgbImage;
use log::warn;
use nalgebra::{
    Affine3, Isometry3, Point3, Scale3, Translation3, UnitQuaternion, Vector2, Vector3,
};
use obj::{ObjMaterial, SimplePolygon};
use ordered_float::OrderedFloat;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Object {
    pub name: String,
    material_name: String,
    path: PathBuf,
    pub triangles: Vec<Triangle>,
    pub materials: Vec<Material>,
    pub translation: Translation3<f32>,
    pub rotation: UnitQuaternion<f32>,
    pub scale: Scale3<f32>,
    bvh: Bvh<f32, 3>,
}

fn load_texture<P: AsRef<Path>>(path: P) -> anyhow::Result<RgbImage> {
    Ok(image::open(path.as_ref())
        .context(format!(
            "Failed to load image from path: {}",
            path.as_ref().display()
        ))?
        .into_rgb8())
}

// extract filename from path and return as String
fn filename<P: AsRef<Path>>(path: P) -> String {
    path.as_ref()
        .file_name()
        .map_or_else(String::new, |s| s.to_string_lossy().to_string())
        .split('.')
        .next()
        .unwrap_or("")
        .to_owned()
        // first char to uppercase
        .chars()
        .enumerate()
        .map(|(i, c)| if i == 0 { c.to_ascii_uppercase() } else { c })
        .collect()
}

impl Object {
    #[expect(
        clippy::panic_in_result_fn,
        reason = "panic if wrong material reference is used"
    )]
    pub fn from_obj<P: AsRef<Path>>(
        path: P,
        translation: Translation3<f32>,
        rotation: UnitQuaternion<f32>,
        scale: Scale3<f32>,
    ) -> anyhow::Result<Self> {
        let mut obj = obj::Obj::load(path.as_ref()).context(format!(
            "Failed to load obj from path: {}",
            path.as_ref().display()
        ))?;

        obj.load_mtls().context(format!(
            "Failed to load materials from obj path: {}",
            path.as_ref().display()
        ))?;

        let materials = obj
            .data
            .material_libs
            .iter()
            .flat_map(|m| &m.materials)
            .map(|m| Material {
                name: m.name.clone(),
                diffuse_color: m.kd.map(Color::from),
                specular_color: m.ks.map(Color::from),
                specular_exponent: m.ns,
                diffuse_texture: {
                    m.map_kd
                        .as_deref()
                        .and_then(|p| path.as_ref().parent().map(|pa| pa.join(p)))
                        .and_then(|p| {
                            load_texture(p.as_path())
                                .map_err(|e| {
                                    warn!("Failed to load texture from path: {}: {e}", p.display());
                                })
                                .ok()
                        })
                },
                illumination_model: m
                    .illum
                    .and_then(IlluminationModel::from_i32)
                    .unwrap_or_else(|| {
                        warn!("Invalid illumination model: {}", m.illum.unwrap_or(-1));
                        IlluminationModel::default()
                    }),
                dissolve: m.d.map(|d| 1.0 - d),
                refraction_index: m.ni,
            })
            .collect::<Vec<_>>();
        let mut warnings = (0, 0, 0);
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
                        ObjMaterial::Ref(str) => {
                            panic!("Material reference not supported: {str}")
                        }
                        ObjMaterial::Mtl(m) => m,
                    })
                    .and_then(|m| {
                        materials
                            .iter()
                            .position(|mat| mat.name == m.name)
                            .or_else(|| {
                                warn!("Material not found: {}", m.name);
                                None
                            })
                    });

                group
                    .polys
                    .iter()
                    .flat_map(|p| triangulate(&obj, p, material_index, &mut warnings))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        if warnings.0 > 0 {
            warn!("Computed normals for {} triangles is zero", warnings.0);
        }

        if warnings.1 > 0 {
            warn!("No normals for {} triangles", warnings.1);
        }

        if warnings.2 > 0 {
            warn!("No UV for {} triangles", warnings.2);
        }

        let bvh = Bvh::build(triangles.as_mut_slice());

        Ok(Self {
            name: filename(&path),
            material_name: obj
                .data
                .objects
                .iter()
                .map(|o| o.name.clone())
                .collect::<Vec<_>>()
                .join(", "),
            path: path.as_ref().to_path_buf(),
            triangles,
            materials,
            translation,
            rotation,
            scale,
            bvh,
        })
    }

    pub fn transform(&self) -> Affine3<f32> {
        Affine3::from_matrix_unchecked(
            Isometry3::from_parts(self.translation, self.rotation).to_homogeneous()
                * self.scale.to_homogeneous(),
        )
    }

    pub fn intersect(&self, ray: Ray, delta: f32) -> Option<Hit<'_>> {
        // Transform ray into object space
        let ray = Ray {
            origin: self.transform().inverse_transform_point(&ray.origin),
            direction: self.transform().inverse_transform_vector(&ray.direction),
        };

        self.bvh
            .traverse(
                &ray::Ray::new(ray.origin, ray.direction),
                self.triangles.as_slice(),
            )
            .into_iter()
            .filter_map(|t| t.intersect(ray, delta).map(|h| (t, h)))
            .map(|(t, (u, v, w))| {
                // u, v, w are barycentric coordinates of the hit point on the triangle
                // interpolate hit point and normal
                let point = Point3::from((t.a * u).coords + (t.b * v).coords + (t.c * w).coords);
                let normal = (t.a_normal * u) + (t.b_normal * v) + (t.c_normal * w);
                let uv = (t.a_uv * u) + (t.b_uv * v) + (t.c_uv * w);
                (t, point, normal, uv)
            })
            .min_by_key(|&(_, point, _, _)| OrderedFloat((ray.origin - point).norm_squared()))
            .map(|(t, point, normal, uv)| {
                // Transform hit point and normal back into world space
                let point = self.transform().transform_point(&point);
                let normal = self.transform().transform_vector(&normal);

                Hit {
                    name: self.material_name.as_str(),
                    point,
                    normal,
                    material: t.material_index.map(|i| &self.materials[i]),
                    uv,
                }
            })
    }
}

/// Triangulate a polygon and compute normals and uv coordinates if they are missing
fn triangulate(
    obj: &obj::Obj,
    poly: &SimplePolygon,
    material_index: Option<usize>,
    (computed_normals_zero, no_normals, no_uv): &mut (u32, u32, u32),
) -> Vec<Triangle> {
    let mut triangles = Vec::new();

    for i in 1..poly.0.len() - 1 {
        let a = Point3::from(obj.data.position[poly.0[0].0]);
        let b = Point3::from(obj.data.position[poly.0[i].0]);
        let c = Point3::from(obj.data.position[poly.0[i + 1].0]);

        let computed_normal = (a - b)
            .cross(&(a - c))
            .try_normalize(f32::EPSILON)
            .unwrap_or_else(|| {
                *computed_normals_zero += 1;
                Vector3::new(0.0, 0.0, 0.0)
            });

        triangles.push(Triangle::new(
            a,
            b,
            c,
            poly.0[0].2.map_or_else(
                || {
                    *no_normals += 1;
                    computed_normal
                },
                |i| Vector3::from(obj.data.normal[i]),
            ),
            poly.0[i].2.map_or_else(
                || {
                    *no_normals += 1;
                    computed_normal
                },
                |i| Vector3::from(obj.data.normal[i]),
            ),
            poly.0[i + 1].2.map_or_else(
                || {
                    *no_normals += 1;
                    computed_normal
                },
                |i| Vector3::from(obj.data.normal[i]),
            ),
            poly.0[0].1.map_or_else(
                || {
                    *no_uv += 1;
                    Vector2::new(0.0, 0.0)
                },
                |i| Vector2::from(obj.data.texture[i]),
            ),
            poly.0[i].1.map_or_else(
                || {
                    *no_uv += 1;
                    Vector2::new(0.0, 0.0)
                },
                |i| Vector2::from(obj.data.texture[i]),
            ),
            poly.0[i + 1].1.map_or_else(
                || {
                    *no_uv += 1;
                    Vector2::new(0.0, 0.0)
                },
                |i| Vector2::from(obj.data.texture[i]),
            ),
            material_index,
        ));
    }

    triangles
}

pub struct WithRelativePath<P: AsRef<Path>>(pub P);

mod yaml {
    use std::{
        f32,
        path::{Path, PathBuf},
    };

    use nalgebra::{Point3, Scale3, Translation3, UnitQuaternion, Vector3};
    use serde::{
        Deserialize, Serialize,
        de::{DeserializeSeed, Error},
    };

    use super::{Object, WithRelativePath};

    #[derive(Serialize, Deserialize)]
    pub struct ObjectDef {
        #[serde(rename = "filePath")]
        pub file_path: PathBuf,
        #[serde(with = "super::super::yaml::point")]
        pub position: Point3<f32>,
        #[serde(with = "super::super::yaml::vector")]
        pub rotation: Vector3<f32>,
        #[serde(with = "super::super::yaml::vector")]
        pub scale: Vector3<f32>,
    }

    impl<'de, P: AsRef<Path>> DeserializeSeed<'de> for WithRelativePath<P> {
        type Value = Object;

        fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let yaml_object = ObjectDef::deserialize(deserializer)?;

            let translation = Translation3::from(yaml_object.position.coords);
            let rotation = UnitQuaternion::from_euler_angles(
                yaml_object.rotation.x * f32::consts::PI * 2.0 / 360.0,
                yaml_object.rotation.y * f32::consts::PI * 2.0 / 360.0,
                yaml_object.rotation.z * f32::consts::PI * 2.0 / 360.0,
            );
            let scale = Scale3::from(yaml_object.scale);

            let path = self
                .0
                .as_ref()
                .parent()
                .map(|p| p.join(yaml_object.file_path.as_path()))
                .ok_or_else(|| Error::custom("Failed to get parent path"))?;

            Object::from_obj(path, translation, rotation, scale)
                .map_err(Error::custom)
                .map(|mut o| {
                    o.path = yaml_object.file_path;
                    o
                })
        }
    }

    impl Serialize for Object {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let rotation = self.rotation.euler_angles();

            ObjectDef {
                file_path: self.path.clone(),
                position: self.translation.vector.into(),
                rotation: Vector3::new(
                    rotation.0.to_degrees(),
                    rotation.1.to_degrees(),
                    rotation.2.to_degrees(),
                ),
                scale: self.scale.vector,
            }
            .serialize(serializer)
        }
    }
}
