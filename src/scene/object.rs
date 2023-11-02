// Import necessary Rust crates and modules
use anyhow::Context;           // Error handling and context for `anyhow` crate
use log::warn;                  // Logging for warnings
use nalgebra::{Point, Similarity3, Vector3}; // Linear algebra utilities
use obj::Material;              // Material from the `obj` crate
use super::triangle::Triangle; // Custom Triangle struct

// Define a custom struct called 'Object'
#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    triangles: Vec<Triangle>,     // A vector of triangles that make up the object
    materials: Vec<Material>,    // A vector of materials associated with the object
    transform: Similarity3<f32>, // A similarity transform for the object
}

// Implement a custom deserializer for 'Object'
impl<'de> serde::Deserialize<'de> for Object {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Define a nested module 'yaml' for deserializing YAML data
        mod yaml {
            use nalgebra::{Point3, Vector3};
            use serde::Deserialize;

            // Define a struct to represent YAML data for 'Object'
            #[derive(Deserialize)]
            pub struct Object {
                pub file_path: String, // File path for the object
                // Deserialize position as a Point3 using a custom deserializer
                #[serde(with = "super::super::yaml::point3_xyz")]
                pub position: Point3<f32>,
                // Deserialize rotation as a Vector3 using a custom deserializer
                #[serde(with = "super::super::yaml::vector3_xyz")]
                pub rotation: Vector3<f32>,
                pub scale: f32, // Scale factor
            }
        }

        // Deserialize YAML data into 'yaml_object'
        let yaml_object = yaml::Object::deserialize(deserializer)?;

        // Create a transformation based on position, rotation, and scale
        let transform = Similarity3::from_parts(
            nalgebra::Translation3::from(yaml_object.position.coords),
            nalgebra::UnitQuaternion::from_euler_angles(
                yaml_object.rotation.x,
                yaml_object.rotation.y,
                yaml_object.rotation.z,
            ),
            yaml_object.scale,
        );

        // Create an 'Object' from an OBJ file using the specified transform
        Object::from_obj(yaml_object.file_path.as_str(), transform)
            .context(format!(
                "Failed to load object from path: {:?}",
                yaml_object.file_path
            ))
            .map_err(serde::de::Error::custom)
    }
}

// Implement methods for the 'Object' struct
impl Object {
    // Define a method to create an 'Object' from an OBJ file
    fn from_obj<P: AsRef<std::path::Path>>(
        path: P,
        transform: Similarity3<f32>,
    ) -> anyhow::Result<Object> {
        // Load the OBJ file and materials
        let mut obj = obj::Obj::load(path.as_ref())
            .context(format!("Failed to load obj from path: {:?}", path.as_ref()))?;
        obj.load_mtls().context(format!(
            "Failed to load materials from obj path: {:?}",
            path.as_ref()
        ))?;

        // Extract materials from the loaded OBJ
        let materials: Vec<Material> = obj
            .data
            .material_libs
            .iter()
            .flat_map(|m| &m.materials)
            .map(|m| m.as_ref().clone())
            .collect::<Vec<_>>();

        // Extract triangles from the loaded OBJ
        let triangles = obj
            .data
            .objects
            .iter()
            .flat_map(|object| object.groups.iter())
            .flat_map(|group| {
                // Determine the material index for the group
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

                // Extract triangles for the group
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

        // Create and return the 'Object' instance
        Ok(Object {
            triangles,
            materials,
            transform,
        })
    }
}
