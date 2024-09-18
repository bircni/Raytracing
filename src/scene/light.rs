use super::Color;
use nalgebra::Point3;

#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct Light {
    pub position: Point3<f32>,
    pub color: Color,
    pub intensity: f32,
}

mod yaml {
    use super::super::Color;
    use super::Light;
    use nalgebra::Point3;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    pub struct LightDef {
        #[serde(with = "super::super::yaml::point")]
        pub position: Point3<f32>,
        #[serde(with = "super::super::yaml::color", rename = "Ke")]
        pub ke: Color,
        pub intensity: f32,
    }

    impl<'de> Deserialize<'de> for Light {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            LightDef::deserialize(deserializer).map(|yaml_light| Self {
                position: yaml_light.position,
                color: yaml_light.ke.try_normalize(0.0).unwrap_or_default(),
                intensity: yaml_light.intensity,
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
                ke: self.color,
                intensity: self.intensity,
            }
            .serialize(serializer)
        }
    }
}
