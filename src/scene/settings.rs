use serde::{Deserialize, Serialize};

use crate::Color;
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Settings {
    pub max_bounces: u32,
    pub samples: u32,
    pub background_color: Color,
    pub ambient_color: Color,
    pub ambient_intensity: f32,
}

impl<'de> Deserialize<'de> for Settings {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        mod yaml {

            use serde::Deserialize;

            use crate::Color;

            #[derive(Deserialize)]
            pub struct Settings {
                pub max_bounces: u32,
                pub samples: u32,
                #[serde(with = "super::super::yaml::color_rgb")]
                pub background_color: Color,
                #[serde(with = "super::super::yaml::color_rgb")]
                pub ambient_color: Color,
            }
        }

        yaml::Settings::deserialize(deserializer).map(|yaml_extras| Settings {
            max_bounces: yaml_extras.max_bounces,
            samples: yaml_extras.samples,
            background_color: yaml_extras.background_color,
            ambient_color: yaml_extras
                .ambient_color
                .try_normalize(0.0)
                .unwrap_or_default(),
            ambient_intensity: yaml_extras.ambient_color.norm(),
        })
    }
}
