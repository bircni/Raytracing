
use nalgebra::{Translation3, UnitQuaternion, Point, Vector3};
use obj::Obj;
use crate::scene::triangle;

use super::triangle::Triangle;

#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    triangles: Vec<Triangle>,
    materials: Vec<obj::Material>,
    translation: Translation3<f32>,
    rotation: UnitQuaternion<f32>,
}

pub fn load_obj_file(file_path: &str) {
    
    // Load the OBJ file
    let mut obj = obj::Obj::load(file_path).unwrap();
    obj.load_mtls().unwrap();
    
    let objects = obj.data.objects.iter().map(|object| {
        object.groups.iter().map(|group| {
            //let material = group.material.clone();
            let triangles = group.polys.iter().map(|poly| {
                
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
                    material_index: 0,
                }

            }).collect::<Vec<_>>();

            Object {
                triangles,
                materials: vec![],
                translation: Translation3::identity(),
                rotation: UnitQuaternion::identity(),
            }

        }).collect::<Vec<_>>()

    }).collect::<Vec<_>>();

    println!("{:?}", objects);
}