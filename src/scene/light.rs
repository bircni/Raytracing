use bytemuck::{Pod, Zeroable};
use nalgebra::Point3;
use serde::Deserialize;

use crate::Color;

#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct Light {
    pub position: Point3<f32>,
    _padding: u32,
    pub color: Color,
    pub intensity: f32,
}

impl<'de> Deserialize<'de> for Light {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        mod yaml {
            use nalgebra::Point3;
            use serde::Deserialize;

            use crate::Color;

            #[derive(Deserialize)]
            pub struct Light {
                #[serde(with = "super::super::yaml::point3_xyz")]
                pub position: Point3<f32>,
                #[serde(with = "super::super::yaml::color_rgb")]
                pub ke: Color,
            }
        }

        yaml::Light::deserialize(deserializer).map(|yaml_light| Light {
            position: yaml_light.position,
            color: yaml_light.ke.normalize(),
            intensity: yaml_light.ke.norm(),
            ..Default::default()
        })
    }
}

#[allow(unsafe_code)]
unsafe impl Zeroable for Light {}
#[allow(unsafe_code)]
unsafe impl Pod for Light {}
