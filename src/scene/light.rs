use nalgebra::Point3;
use serde::Deserialize;

use crate::Color;

#[derive(Debug, Clone, PartialEq)]
pub struct Light {
    position: Point3<f32>,
    color: Color,
    intensity: f32,
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
        })
    }
}
