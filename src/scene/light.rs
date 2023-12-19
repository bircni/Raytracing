use nalgebra::Point3;

use crate::Color;

#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct Light {
    pub position: Point3<f32>,
    pub color: Color,
    pub intensity: f32,
}

mod yaml {
    use nalgebra::Point3;
    use serde::{Deserialize, Serialize};

    use crate::Color;

    use super::Light;

    #[derive(Serialize, Deserialize)]
    pub struct LightDef {
        #[serde(with = "super::super::yaml::point")]
        pub position: Point3<f32>,
        #[serde(with = "super::super::yaml::color", rename = "Ke")]
        pub ke: Color,
    }

    impl<'de> Deserialize<'de> for Light {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            LightDef::deserialize(deserializer).map(|yaml_light| Light {
                position: yaml_light.position,
                color: yaml_light.ke.try_normalize(0.0).unwrap_or_default(),
                intensity: yaml_light.ke.norm(),
            })
        }
    }

    impl Serialize for Light {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            LightDef {
                position: self.position,
                ke: self.color * self.intensity,
            }
            .serialize(serializer)
        }
    }
}
